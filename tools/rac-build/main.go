package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/pelletier/go-toml/v2"
)

const (
	cargoTomlFilename = "Cargo.toml"
	releaseTargetDir  = "target/release"
	tempFileSuffix    = ".tmp"
)

type CargoConfig struct {
	Package PackageInfo `toml:"package"`
}

type PackageInfo struct {
	Name    string `toml:"name"`
	Version string `toml:"version"`
}

type BuildOptions struct {
	Verbose   bool
	Clean     bool
	OutputDir string
	CargoPath string
	DryRun    bool
}

type BuildContext struct {
	Options    BuildOptions
	ProjectDir string
	Config     *CargoConfig
	CargoBin   string
}

func main() {
	log.SetFlags(log.Ltime)

	opts := parseCLIFlags()

	if err := execute(opts); err != nil {
		log.Fatalf("Build failed: %v", err)
	}
}

func parseCLIFlags() BuildOptions {
	opts := BuildOptions{}

	flag.BoolVar(&opts.Verbose, "v", false, "Enable verbose output")
	flag.BoolVar(&opts.Verbose, "verbose", false, "Enable verbose output")
	flag.BoolVar(&opts.Clean, "clean", false, "Clean before building")
	flag.StringVar(&opts.OutputDir, "o", "", "Custom output directory (default: target/release)")
	flag.StringVar(&opts.CargoPath, "cargo-path", "", "Explicit cargo binary path")
	flag.BoolVar(&opts.DryRun, "dry-run", false, "Show planned actions without executing")

	flag.Parse()

	return opts
}

func execute(opts BuildOptions) error {
	ctx := context.Background()

	projectDir, err := locateProjectRoot()
	if err != nil {
		return fmt.Errorf("project discovery: %w", err)
	}

	log.Printf("Project root: %s", projectDir)

	config, err := loadCargoConfig(projectDir)
	if err != nil {
		return fmt.Errorf("config loading: %w", err)
	}

	if err := config.Validate(); err != nil {
		return fmt.Errorf("config validation: %w", err)
	}

	if err := validateVersionString(config.Package.Version); err != nil {
		return fmt.Errorf("version validation: %w", err)
	}

	cargoBin, err := resolveCargoBinary(opts.CargoPath)
	if err != nil {
		return fmt.Errorf("cargo resolution: %w", err)
	}

	if err := validateCargoBinary(cargoBin); err != nil {
		return fmt.Errorf("cargo validation: %w", err)
	}

	buildCtx := &BuildContext{
		Options:    opts,
		ProjectDir: projectDir,
		Config:     config,
		CargoBin:   cargoBin,
	}

	log.Printf("Building %s v%s", config.Package.Name, config.Package.Version)

	if err := runBuildPipeline(ctx, buildCtx); err != nil {
		return fmt.Errorf("build pipeline: %w", err)
	}

	return nil
}

func runBuildPipeline(ctx context.Context, buildCtx *BuildContext) error {
	if err := os.Chdir(buildCtx.ProjectDir); err != nil {
		return fmt.Errorf("chdir to project: %w", err)
	}

	if buildCtx.Options.Clean {
		if err := executeCargoClean(ctx, buildCtx); err != nil {
			return fmt.Errorf("cargo clean: %w", err)
		}
	}

	if err := executeCargoBuild(ctx, buildCtx); err != nil {
		return fmt.Errorf("cargo build: %w", err)
	}

	builtBinary, err := locateBuiltBinary(buildCtx)
	if err != nil {
		return fmt.Errorf("binary location: %w", err)
	}

	if err := copyBinaryToDestination(buildCtx, builtBinary); err != nil {
		return fmt.Errorf("binary copy: %w", err)
	}

	return nil
}

func locateProjectRoot() (string, error) {
	cwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("get working directory: %w", err)
	}

	currentDir := cwd

	for {
		cargoTomlPath := filepath.Join(currentDir, cargoTomlFilename)

		if fileExists(cargoTomlPath) {
			return currentDir, nil
		}

		parentDir := filepath.Dir(currentDir)

		if parentDir == currentDir {
			break
		}

		currentDir = parentDir
	}

	return "", errors.New("cargo.toml not found in current or parent directories")
}

func loadCargoConfig(projectDir string) (*CargoConfig, error) {
	cargoTomlPath := filepath.Join(projectDir, cargoTomlFilename)

	data, err := os.ReadFile(cargoTomlPath)
	if err != nil {
		return nil, fmt.Errorf("read Cargo.toml: %w", err)
	}

	configuration := CargoConfig{}
	if err := toml.Unmarshal(data, &configuration); err != nil {
		return nil, fmt.Errorf("parse Cargo.toml: %w", err)
	}

	return &configuration, nil
}

func (c *CargoConfig) Validate() error {
	if c.Package.Name == "" {
		return errors.New("package.name is empty in Cargo.toml")
	}
	if c.Package.Version == "" {
		return errors.New("package.version is empty in Cargo.toml")
	}
	return nil
}

func validateVersionString(version string) error {
	if version == "" {
		return errors.New("version string is empty")
	}

	forbiddenChars := `/\:"<>|?*` + "\x00"

	if strings.ContainsAny(version, forbiddenChars) {
		return fmt.Errorf("version contains forbidden characters: %q", version)
	}

	for _, r := range version {
		if r < 32 || r == 127 {
			return fmt.Errorf("version contains control character: %q", version)
		}
	}

	return nil
}

func validateCargoBinary(cargo string) error {
	cmd := exec.Command(cargo, "--version")
	out, err := cmd.Output()
	if err != nil {
		return errors.New("unable to execute cargo --version")
	}

	if !strings.HasPrefix(string(out), "cargo ") {
		return errors.New("invalid cargo binary")
	}
	return nil
}

func resolveCargoBinary(explicit string) (string, error) {
	if explicit != "" {
		return filepath.Abs(explicit)
	}
	return exec.LookPath("cargo")
}

func executeCargoClean(ctx context.Context, buildCtx *BuildContext) error {
	log.Println("Running: cargo clean")

	if buildCtx.Options.DryRun {
		log.Println("[DRY-RUN] Skipping cargo clean")
		return nil
	}

	return runCargoCommand(ctx, buildCtx, "clean")
}

func executeCargoBuild(ctx context.Context, buildCtx *BuildContext) error {
	log.Println("Running: cargo build --release")

	if buildCtx.Options.DryRun {
		log.Println("[DRY-RUN] Skipping cargo build")
		return nil
	}

	return runCargoCommand(ctx, buildCtx, "build", "--release")
}

func runCargoCommand(ctx context.Context, buildCtx *BuildContext, args ...string) error {
	cmd := exec.CommandContext(ctx, buildCtx.CargoBin, args...)
	cmd.Dir = buildCtx.ProjectDir

	if buildCtx.Options.Verbose {
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
	} else {
		capturedOutput := strings.Builder{}
		cmd.Stdout = &capturedOutput
		cmd.Stderr = &capturedOutput

		if err := cmd.Run(); err != nil {
			if _, writeErr := fmt.Fprintln(os.Stderr, capturedOutput.String()); writeErr != nil {
				log.Printf("Warning: failed to write error output: %v", writeErr)
			}
			return fmt.Errorf("cargo command failed: %w", err)
		}

		return nil
	}

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("cargo command failed: %w", err)
	}

	return nil
}

func locateBuiltBinary(buildCtx *BuildContext) (string, error) {
	binaryName := buildCtx.Config.Package.Name + getPlatformExecutableExtension()
	binaryPath := filepath.Join(buildCtx.ProjectDir, releaseTargetDir, binaryName)

	if !fileExists(binaryPath) {
		return "", fmt.Errorf("binary not found at expected location: %s", binaryPath)
	}

	return binaryPath, nil
}

func copyBinaryToDestination(buildCtx *BuildContext, sourceBinary string) error {
	outputDir := releaseTargetDir
	if buildCtx.Options.OutputDir != "" {
		outputDir = buildCtx.Options.OutputDir
	}

	absOutputDir, err := filepath.Abs(outputDir)
	if err != nil {
		return fmt.Errorf("resolve output directory: %w", err)
	}

	if !buildCtx.Options.DryRun {
		if err := os.MkdirAll(absOutputDir, 0755); err != nil {
			return fmt.Errorf("create output directory: %w", err)
		}
	}

	destFilename := fmt.Sprintf("%s-v%s%s",
		buildCtx.Config.Package.Name,
		buildCtx.Config.Package.Version,
		getPlatformExecutableExtension())

	destPath := filepath.Join(absOutputDir, destFilename)

	if buildCtx.Options.DryRun {
		log.Printf("[DRY-RUN] Would copy: %s -> %s", sourceBinary, destPath)
		return nil
	}

	if err := performFileCopy(sourceBinary, destPath); err != nil {
		return fmt.Errorf("copy binary: %w", err)
	}

	fileInfo, err := os.Stat(destPath)
	if err != nil {
		return fmt.Errorf("stat destination file: %w", err)
	}

	log.Printf("✓ SUCCESS: %s (%d bytes)", destPath, fileInfo.Size())

	return nil
}

func performFileCopy(source, destination string) error {
	sourceFile, err := os.Open(source)
	if err != nil {
		return fmt.Errorf("open source: %w", err)
	}
	defer func() {
		if closeErr := sourceFile.Close(); closeErr != nil {
			log.Printf("Warning: failed to close source file: %v", closeErr)
		}
	}()

	tempPath := destination + tempFileSuffix

	destFile, err := os.OpenFile(tempPath, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0600)
	if err != nil {
		return fmt.Errorf("create temp file: %w", err)
	}

	operationSuccessful := false
	closed := false
	defer func() {
		if !closed {
			if closeErr := destFile.Close(); closeErr != nil && operationSuccessful {
				log.Printf("Warning: failed to close destination file: %v", closeErr)
			}
		}
		if !operationSuccessful {
			if removeErr := os.Remove(tempPath); removeErr != nil {
				log.Printf("Warning: failed to remove temporary file: %v", removeErr)
			}
		}
	}()

	if _, err := io.Copy(destFile, sourceFile); err != nil {
		return fmt.Errorf("copy data: %w", err)
	}

	if err := destFile.Sync(); err != nil {
		return fmt.Errorf("fsync file: %w", err)
	}

	if err := destFile.Close(); err != nil {
		return fmt.Errorf("close temp file: %w", err)
	}

	closed = true

	if err := os.Rename(tempPath, destination); err != nil {
		return fmt.Errorf("atomic rename: %w", err)
	}

	operationSuccessful = true
	return nil
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil || !errors.Is(err, os.ErrNotExist)
}

func getPlatformExecutableExtension() string {
	if runtime.GOOS == "windows" {
		return ".exe"
	}
	return ""
}

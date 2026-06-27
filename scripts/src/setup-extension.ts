import { Command } from "commander";
import { mkdir, cp, rm } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { existsSync } from "fs";

async function setup() {
  const program = new Command();
  program
    .name("setup-extension")
    .description("Install Galfus Script syntax highlighting extension for VSCode/Antigravity")
    .option("-v, --vscode", "Install to VS Code and VS Code Insiders")
    .option("-a, --antigravity", "Install to Antigravity IDE")
    .option("--all", "Install to all editors (default)")
    .parse(process.argv);

  const options = program.opts();
  const allTargets = options.all || (!options.vscode && !options.antigravity);

  const home = homedir();
  const targets: { name: string; path: string }[] = [];

  if (allTargets || options.vscode) {
    targets.push(
      { name: "VS Code", path: join(home, ".vscode", "extensions", "galfus-vscode") },
      { name: "VS Code Insiders", path: join(home, ".vscode-insiders", "extensions", "galfus-vscode") }
    );
  }

  if (allTargets || options.antigravity) {
    targets.push(
      { name: "Antigravity", path: join(home, ".antigravity", "extensions", "galfus-vscode") }
    );
  }

  const sourceDir = join(__dirname, "..", "..", "editors", "vscode");

  let installedCount = 0;

  for (const target of targets) {
    const parentDir = join(target.path, "..");
    if (existsSync(parentDir)) {
      console.log(`Installing extension to ${target.name} (${target.path})...`);
      try {
        if (existsSync(target.path)) {
          await rm(target.path, { recursive: true, force: true });
        }
        await mkdir(target.path, { recursive: true });
        await cp(sourceDir, target.path, { recursive: true });
        console.log(`Successfully installed to ${target.name}`);
        installedCount++;
      } catch (err) {
        console.error(`Failed to install to ${target.name}:`, err);
      }
    }
  }

  if (installedCount === 0) {
    console.warn("Could not find standard editor extension directories.");
    console.info(`Please copy the 'editors/vscode' folder manually to your editor's extension folder.`);
  } else {
    console.log(`Setup complete! Please restart or reload your editor to apply syntax highlighting.`);
  }
}

setup().catch(console.error);

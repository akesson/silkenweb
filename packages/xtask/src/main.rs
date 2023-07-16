use std::{
    ffi::OsStr,
    fmt::Write,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use itertools::Itertools;
use scopeguard::defer;
use xshell::{cmd, mkdir_p, pushd, read_dir, rm_rf, write_file};
use xtask_base::{
    build_readme, ci_nightly, clippy, generate_open_source_files, run, CommonCmds, WorkflowResult,
};

#[derive(Parser)]
enum Commands {
    /// Generate all derived files. Will overwrite existing content.
    Codegen {
        /// If set, just check the file contents are up to date.
        #[clap(long)]
        check: bool,
    },
    /// Run CI checks
    Ci {
        #[clap(subcommand)]
        command: Option<CiCommand>,
    },
    TestFeatures,
    WasmPackTest,
    TrunkBuild,
    /// Run TodoMVC with `trunk`
    TodomvcRun,
    /// Run the TodoMVC Cypress tests
    TodomvcCypress {
        #[clap(long)]
        gui: bool,
    },
    BuildWebsite,
    GithubActions {
        #[clap(long)]
        full: bool,
    },
    #[clap(flatten)]
    Common(CommonCmds),
}

#[derive(Subcommand, PartialEq, Eq)]
enum CiCommand {
    Stable {
        #[clap(long)]
        fast: bool,
        toolchain: Option<String>,
    },
    Nightly {
        toolchain: Option<String>,
    },
    Browser,
}

fn main() {
    run(|workspace| {
        match Commands::parse() {
            Commands::Codegen { check } => {
                build_readme(".", check)?;
                generate_open_source_files(2021, check)?;
            }
            Commands::Ci { command } => {
                if let Some(command) = command {
                    match command {
                        CiCommand::Stable { fast, toolchain } => ci_stable(fast, toolchain)?,
                        CiCommand::Nightly { toolchain } => ci_nightly(toolchain.as_deref())?,
                        CiCommand::Browser => ci_browser()?,
                    }
                } else {
                    ci_stable(false, None)?;
                    ci_nightly(Some("nightly"))?;
                    ci_browser()?;
                    wasm_pack_test()?;
                }
            }
            Commands::TestFeatures => test_features()?,
            Commands::WasmPackTest => wasm_pack_test()?,
            Commands::TrunkBuild => trunk_build()?,
            Commands::TodomvcRun => {
                let _dir = pushd("examples/todomvc")?;
                cmd!("trunk serve --open").run()?;
            }
            Commands::TodomvcCypress { gui } => {
                cypress("install", if gui { "open" } else { "run" }, None)?;
            }
            Commands::BuildWebsite => build_website()?,
            Commands::GithubActions { full } => {
                let reuse = (!full).then_some("--reuse");

                cmd!("docker build . -t silkenweb-github-actions").run()?;
                cmd!(
                    "act -P ubuntu-latest=silkenweb-github-actions:latest --use-gitignore {reuse...}"
                )
                .run()?;
            }
            Commands::Common(cmds) => cmds.run::<Commands>(workspace)?,
        }

        Ok(())
    });
}

fn build_website() -> WorkflowResult<()> {
    let dest_dir = "target/website";
    rm_rf(dest_dir)?;
    let examples_dest_dir = format!("{dest_dir}/examples");
    mkdir_p(&examples_dest_dir)?;
    let mut redirects = String::new();

    for example in browser_examples()? {
        let examples_dir: PathBuf = [Path::new("examples"), &example].iter().collect();

        {
            let _dir = pushd(&examples_dir);
            cmd!("trunk build --release --public-url examples/{example}").run()?;
        }

        cmd!("cp -R {examples_dir}/dist/ {examples_dest_dir}/{example}").run()?;

        {
            let example = example.display();
            writeln!(
                &mut redirects,
                "/examples/{example}/* /examples/{example}/index.html 200"
            )?;
        }
    }

    write_file(format!("{dest_dir}/_redirects"), redirects)?;

    Ok(())
}

fn ci_browser() -> WorkflowResult<()> {
    cypress("ci", "run", None)?;
    trunk_build()
}

fn ci_stable(fast: bool, toolchain: Option<String>) -> WorkflowResult<()> {
    build_readme(".", true)?;
    generate_open_source_files(2021, true)?;
    xtask_base::ci_stable(fast, toolchain.as_deref(), &[])?;
    test_features()
}

fn test_features() -> WorkflowResult<()> {
    for features in ["declarative-shadow-dom"].into_iter().powerset() {
        if !features.is_empty() {
            clippy(None, &features)?;

            let features = features.join(",");

            cmd!("cargo test --package silkenweb --features {features}").run()?;
        }
    }

    Ok(())
}

fn cypress(npm_install_cmd: &str, cypress_cmd: &str, browser: Option<&str>) -> WorkflowResult<()> {
    let _dir = pushd("examples/todomvc")?;
    cmd!("trunk build").run()?;
    let trunk = duct::cmd("trunk", ["serve", "--no-autoreload", "--ignore=."]).start()?;
    defer! { let _ = trunk.kill(); };

    let _dir = pushd("e2e")?;
    cmd!("npm {npm_install_cmd}").run()?;

    if let Some(browser) = browser {
        cmd!("npx cypress {cypress_cmd} --browser {browser}").run()?;
    } else {
        cmd!("npx cypress {cypress_cmd}").run()?;
    }

    Ok(())
}

fn wasm_pack_test() -> WorkflowResult<()> {
    let _dir = pushd("packages/silkenweb")?;
    cmd!("wasm-pack test --headless --firefox").run()?;
    Ok(())
}

fn browser_examples() -> WorkflowResult<Vec<PathBuf>> {
    let _dir = pushd("examples");
    let examples = read_dir(".")?;
    let non_browser = ["htmx-axum"];
    let non_browser: Vec<_> = non_browser
        .into_iter()
        .map(|x| Some(OsStr::new(x)))
        .collect();
    let mut browser_examples = Vec::new();

    for example in examples {
        if !non_browser.contains(&example.file_name()) {
            for file in read_dir(&example)? {
                if file.extension() == Some(OsStr::new("html")) {
                    browser_examples.push(example);
                    break;
                }
            }
        }
    }

    Ok(browser_examples)
}

fn trunk_build() -> WorkflowResult<()> {
    let _dir = pushd("examples");

    for example in browser_examples()? {
        let _dir = pushd(example)?;
        cmd!("trunk build").run()?;
    }

    Ok(())
}

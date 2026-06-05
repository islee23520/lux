use super::{
    codex_hook_status, install_codex_hook_bridge, run::HookRunFailure, run_hook_bridge,
    HooksAction, HooksArgs,
};
use anyhow::Result;

pub fn run_hooks_command(args: HooksArgs) -> Result<()> {
    match args.action {
        HooksAction::Install(install_args) => {
            let report = install_codex_hook_bridge(&install_args)?;
            if install_args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.dry_run {
                eprintln!(
                    "Lux hook install preview for {}:",
                    report.hooks_path.display()
                );
                for event in &report.installed {
                    let marker = if event.already_installed {
                        "already installed"
                    } else {
                        "would install"
                    };
                    eprintln!("  {}: {marker}", event.event);
                }
            } else {
                let changed = if report.changed {
                    "updated"
                } else {
                    "unchanged"
                };
                eprintln!(
                    "Lux hook bridge {changed} at {}",
                    report.hooks_path.display()
                );
            }
            Ok(())
        }
        HooksAction::Status(status_args) => {
            let report = codex_hook_status(&status_args)?;
            if status_args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!(
                    "Lux hook bridge status for {}:",
                    report.hooks_path.display()
                );
                for event in &report.events {
                    let status = if event.installed {
                        "installed"
                    } else {
                        "missing"
                    };
                    eprintln!("  {}: {status}", event.event);
                }
            }
            Ok(())
        }
        HooksAction::Run(run_args) => match run_hook_bridge(&run_args) {
            Ok(report) => {
                if run_args.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                Ok(())
            }
            Err(error) => {
                if run_args.json {
                    if let Some(report) = error.downcast_ref::<HookRunFailure>() {
                        println!("{}", serde_json::to_string_pretty(&report.report)?);
                    }
                }
                Err(error)
            }
        },
    }
}

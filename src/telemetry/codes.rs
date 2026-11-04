// === Error codes

/* Decision 9 as amended 2026-11-04 (schema 3): a failure is reported by the
 * code of the error that caused it, never by its message, which carries the
 * user's paths, project names, and task names. Every error type's `code()`
 * match is exhaustive, so a new variant cannot compile without a code;
 * `tests/telemetry_codes.rs` keeps `ALL` equal to those codes and to the
 * receiver's own list. */

use crate::cas::CasError;
use crate::cli::CliError;
use crate::install::InstallError;
use crate::layer::LayerError;
use crate::merge::MergeError;
use crate::preset::PresetError;
use crate::preset::remote::RemoteError;
use crate::runner::RunnerError;
use crate::runner::remote_cache::RemoteCacheError;
use crate::scaffold::ScaffoldError;

/// Every code that can be sent, sorted.
pub const ALL: &[&str] = &[
    "cas_corrupted",
    "cas_io",
    "cas_malformed",
    "cas_missing",
    "cas_no_cache_dir",
    "cli_cancelled",
    "cli_destination_not_a_directory",
    "cli_destination_not_empty",
    "cli_no_layers_chosen",
    "cli_no_task_or_affected",
    "cli_no_tasks_defined",
    "cli_project_name_empty",
    "cli_project_name_is_path",
    "cli_project_name_starts_with_dash",
    "cli_project_name_unusable",
    "cli_release_age",
    "install_dev_unsupported",
    "install_failed",
    "install_features_unsupported",
    "install_malformed_package_json",
    "install_no_root_package_json",
    "install_not_started",
    "install_unknown_manager",
    "io",
    "layer_category_empty",
    "layer_category_forbidden",
    "layer_category_overfull",
    "layer_conflicting",
    "layer_duplicate_id",
    "layer_malformed",
    "layer_missing_requirement",
    "layer_non_table_merge_value",
    "layer_shape_missing",
    "layer_single_app_has_no_app",
    "layer_unknown_layer",
    "layer_unknown_shape",
    "layer_unqualified_reference",
    "layer_unreadable",
    "merge_conflicting",
    "merge_missing_requirement",
    "merge_non_object_root",
    "merge_scalar_conflict",
    "merge_strategy_mismatch",
    "merge_type_conflict",
    "other",
    "preset_malformed",
    "preset_misnamed",
    "preset_remote_clone_failed",
    "preset_remote_git_not_started",
    "preset_remote_malformed",
    "preset_remote_missing_file",
    "preset_remote_no_cache_dir",
    "preset_unknown",
    "remote_cache_rejected",
    "remote_cache_unreachable",
    "runner_bad_pattern",
    "runner_cache",
    "runner_cache_mismatch",
    "runner_cycle",
    "runner_git_unavailable",
    "runner_malformed",
    "runner_nested_workspace",
    "runner_not_started",
    "runner_output_outside_project",
    "runner_task_failed",
    "runner_unknown_dependency",
    "runner_unknown_project",
    "runner_unknown_ref",
    "runner_unknown_task",
    "runner_unreadable",
    "scaffold_escapes_root",
    "scaffold_file_collision",
    "scaffold_io",
    "scaffold_template_missing",
];

/// The code of the first error in the chain culm recognises, or `other`.
/// The chain is walked because a command usually wraps the real failure in context.
pub fn of(error: &anyhow::Error) -> &'static str {
    for cause in error.chain() {
        if let Some(code) = code_of(cause) {
            return code;
        }
    }
    return "other";
}

fn code_of(cause: &(dyn std::error::Error + 'static)) -> Option<&'static str> {
    if let Some(e) = cause.downcast_ref::<CliError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<LayerError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<MergeError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<ScaffoldError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<RunnerError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<CasError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<PresetError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<RemoteError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<InstallError>() {
        return Some(e.code());
    }
    if let Some(e) = cause.downcast_ref::<RemoteCacheError>() {
        return Some(e.code());
    }
    if cause.downcast_ref::<std::io::Error>().is_some() {
        return Some("io");
    }
    return None;
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn all_is_sorted_and_unique() {
        let mut sorted = ALL.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, ALL);
    }

    #[test]
    fn a_wrapped_error_reports_its_cause() {
        let error = Err::<(), _>(CliError::NoLayersChosen)
            .context("while choosing layers")
            .unwrap_err();
        assert_eq!(of(&error), "cli_no_layers_chosen");
    }

    #[test]
    fn an_unknown_error_is_other() {
        assert_eq!(of(&anyhow::anyhow!("anything at all")), "other");
    }

    #[test]
    fn every_code_is_in_all() {
        assert!(ALL.contains(&CliError::Cancelled.code()));
        assert!(ALL.contains(&of(&anyhow::Error::new(std::io::Error::other("x")))));
    }
}

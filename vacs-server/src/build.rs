use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BuildInfo {
    pub git_sha: &'static str,
    pub git_branch: &'static str,
    pub git_describe: &'static str,
    pub git_commit_date: &'static str,
    pub git_commit_message: &'static str,
    pub git_dirty: &'static str,
    pub build_timestamp: &'static str,
}

impl BuildInfo {
    pub fn gather() -> Self {
        Self {
            git_sha: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
            git_branch: option_env!("VERGEN_GIT_BRANCH").unwrap_or("unknown"),
            git_describe: option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("unknown"),
            git_commit_date: option_env!("VERGEN_GIT_COMMIT_DATE").unwrap_or_default(),
            git_commit_message: option_env!("VERGEN_GIT_COMMIT_MESSAGE").unwrap_or_default(),
            git_dirty: option_env!("VERGEN_GIT_DIRTY").unwrap_or("false"),
            build_timestamp: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or_default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CompilerInfo {
    pub cargo_target_triple: &'static str,
    pub cargo_features: &'static str,
    pub cargo_opt_level: &'static str,
    pub rustc_semver: &'static str,
    pub rustc_channel: &'static str,
    pub rustc_host_triple: &'static str,
}

impl CompilerInfo {
    #[allow(dead_code)]
    pub fn gather() -> Self {
        Self {
            cargo_target_triple: option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or("unknown"),
            cargo_features: option_env!("VERGEN_CARGO_FEATURES").unwrap_or_default(),
            cargo_opt_level: option_env!("VERGEN_CARGO_OPT_LEVEL").unwrap_or_default(),
            rustc_semver: option_env!("VERGEN_RUSTC_SEMVER").unwrap_or("unknown"),
            rustc_channel: option_env!("VERGEN_RUSTC_CHANNEL").unwrap_or("unknown"),
            rustc_host_triple: option_env!("VERGEN_RUSTC_HOST_TRIPLE").unwrap_or("unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub build: BuildInfo,
    pub version: &'static str,
    pub dataset_git_sha: String,
}

impl VersionInfo {
    pub fn gather() -> Self {
        Self {
            build: BuildInfo::gather(),
            version: env!("CARGO_PKG_VERSION"),
            dataset_git_sha: "unknown".to_string(),
        }
    }
}

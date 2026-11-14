use clap::Parser;

use crate::config::{get_config_dir, get_data_dir};

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 1.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 30.0)]
    pub frame_rate: f64,
}

// `cargo install` builds from the crates.io tarball, which never contains a
// `.git` directory, so vergen can't populate VERGEN_GIT_DESCRIBE there. Fall
// back to "unknown" instead of hard-failing the build (see issue #60).
fn version_message() -> String {
    let git_describe = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("unknown");
    format!(
        "{}-{} ({})",
        env!("CARGO_PKG_VERSION"),
        git_describe,
        env!("VERGEN_BUILD_DATE")
    )
}

pub fn version() -> String {
    let author = clap::crate_authors!();

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();
    let version_message = version_message();

    format!(
        "\
{version_message}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tick_rate_lets_the_key_combo_buffer_time_out_within_a_second() {
        let cli = Cli::parse_from(["brt"]);
        assert!(
            cli.tick_rate >= 1.0,
            "tick_rate defaulted to {}, which clears the multi-key combo buffer only once every {:.0} seconds",
            cli.tick_rate,
            1.0 / cli.tick_rate
        );
    }
}

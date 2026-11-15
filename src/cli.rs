use clap::Parser;

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
// `.git` directory, so vergen can't populate VERGEN_GIT_DESCRIBE there — it
// emits the env var as an empty string rather than leaving it unset (see
// issue #60). Treat both "unset" and "empty" as "not available" and just
// omit the suffix instead of showing a placeholder (see issue #62).
pub fn version() -> String {
    match option_env!("VERGEN_GIT_DESCRIBE") {
        Some(git_describe) if !git_describe.is_empty() => {
            format!("{} ({git_describe})", env!("CARGO_PKG_VERSION"))
        }
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
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

// === The startup banner

/* The mark is the Layered C: a letterform "c" built from a segmented stroke,
 * with node gaps at 90 degrees and 210 degrees. The gap in the top arc is the
 * first node; the blank row in the left stroke is the second. Drawn at eight
 * rows so the segmentation survives, since a smaller c loses the gaps and
 * reads as an o. */
const MARK: [&str; 8] = [
    "  ▄▄▄▄ ▄▄",
    "▄█▀▀     ",
    "██       ",
    "██       ",
    "         ",
    "██       ",
    "▀█▄▄     ",
    "  ▀▀▀▀▀▀▀",
];

/* Seven rows against the mark's eight, so it hangs from the second row. Every
 * row is padded to the same width, because the wordmark column has to start at
 * one column on every line and the mark's own rows are ragged. */
const WORDMARK: [&str; 7] = [
    "          ▗▄▖       ",
    "          ▝▜▌       ",
    " ▟██▖▐▌ ▐▌ ▐▌  ▐█▙█▖",
    "▐▛  ▘▐▌ ▐▌ ▐▌  ▐▌█▐▌",
    "▐▌   ▐▌ ▐▌ ▐▌  ▐▌█▐▌",
    "▝█▄▄▌▐▙▄█▌ ▐▙▄ ▐▌█▐▌",
    " ▝▀▀  ▀▀▝▘  ▀▀ ▝▘▀▝▘",
];

/* #016630 is the locked light-variant green and is close to unreadable on a
 * dark terminal, so the ramp is built above it rather than around it. One
 * shade per row, brightest at the top: the mark is called Layered C, and a
 * flat fill draws a letter where a ramp draws a stack. */
const MARK_RAMP: [&str; 8] = [
    "\x1b[38;2;64;222;134m",
    "\x1b[38;2;52;205;122m",
    "\x1b[38;2;42;188;110m",
    "\x1b[38;2;33;171;98m",
    "\x1b[38;2;27;154;86m",
    "\x1b[38;2;20;136;74m",
    "\x1b[38;2;12;119;62m",
    "\x1b[38;2;1;102;48m",
];

/* The wordmark takes the logo's dark variant, white, and cools by a few
 * points per row so it sits in the same light as the mark beside it. */
const WORD_RAMP: [&str; 7] = [
    "\x1b[38;2;255;255;255m",
    "\x1b[38;2;246;251;248m",
    "\x1b[38;2;237;247;241m",
    "\x1b[38;2;228;243;234m",
    "\x1b[38;2;219;239;227m",
    "\x1b[38;2;210;235;220m",
    "\x1b[38;2;201;231;213m",
];

const TAGLINE: &str = "\x1b[38;2;27;154;86m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// Prints the banner, or nothing at all when colour is unwelcome.
/// Called before any command runs, so it is the first thing a user sees.
pub fn print() {
    if suppressed() {
        return;
    }

    let (dim, reset) = (DIM, RESET);

    println!();
    println!("  {}{}{reset}", MARK_RAMP[0], MARK[0]);
    for row in 1..MARK.len() {
        println!(
            "  {}{}{reset}  {}{}{reset}",
            MARK_RAMP[row],
            MARK[row],
            WORD_RAMP[row - 1],
            WORDMARK[row - 1]
        );
    }
    println!();
    println!(
        "   {TAGLINE}a workspace, composed from layers{reset}   {dim}v{}{reset}",
        env!("CARGO_PKG_VERSION")
    );
    println!();
}

/* Three ways to say no, and any one of them is enough. `NO_COLOR` is the
 * cross-tool convention and is honoured whatever its value, per no-color.org.
 * A non-terminal stdout means the output is being piped or captured, where
 * eight rows of block drawing are noise. `CULM_NO_BANNER` is the escape hatch
 * for somebody who wants colour everywhere else and not this. */
fn suppressed() -> bool {
    std::env::var_os("NO_COLOR").is_some()
        || std::env::var_os("CULM_NO_BANNER").is_some()
        || !std::io::IsTerminal::is_terminal(&std::io::stdout())
}

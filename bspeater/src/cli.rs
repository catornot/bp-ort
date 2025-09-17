use clap::Parser;
use std::path::PathBuf;

const PATH: &str = "/home/catornot/.local/share/Steam/steamapps/common/Titanfall2/vpk/";

#[derive(Parser)] // requires `derive` feature
#[command(name = "bspeater")]
#[command(bin_name = "bspeater")]
pub struct BspeaterCli {
    #[arg(long, short = 'v', default_value = "target")]
    pub vpk_dir: PathBuf,

    #[arg(long, short = 'd', default_value = PATH)]
    pub game_dir: PathBuf,

    #[arg(long, short = 's', default_value_t = true, action = clap::ArgAction::SetFalse)]
    pub display: bool,

    #[arg(long, short = 'n')]
    pub map_name: String,

    #[arg(long, short = 't', default_value_t = false, action = clap::ArgAction::SetTrue)]
    pub show_octtree: bool,

    #[arg(long, short = 'o', default_value = "output")]
    pub output: PathBuf,
}

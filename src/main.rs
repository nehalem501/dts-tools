use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand};

use crate::extract::{Feature, FeatureId, FeatureName, TrailerIds, TrailerNames, Trailers};

mod bcd;
mod cd;
mod detect;
mod ext234;
mod ext234file;
mod extract;
mod file;
mod hdd;
mod hdr;
mod info;
mod iso;
mod isofile;
mod json;
mod metadata;
mod osfile;
mod partitionfile;
mod snd;
mod squash;
mod squashfsfile;
mod trailers;

#[derive(Parser)]
#[command(version)]
#[command(name = "dts")]
#[command(about = "DTS tools", long_about = None)]
struct Cli {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
#[group(required = false, multiple = false)]
pub struct FeatureGroup {
    #[arg(long)]
    feature_name: Option<String>,

    #[arg(long)]
    feature_id: Option<u16>,
}

#[derive(Args)]
#[group(required = false, multiple = false)]
pub struct TrailersGroup {
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    trailer_names: Option<Vec<String>>,

    #[arg(long, num_args = 1.., value_delimiter = ',')]
    trailer_ids: Option<Vec<u16>>,
}

#[derive(Debug, Args)]
struct GlobalOpts {
    #[clap(long, short, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Info {
        //#[arg(arg_required_else_help = true)]
        file: Vec<PathBuf>,

        #[arg(long)]
        output_json: Option<PathBuf>,
    },
    Extract {
        //#[arg(arg_required_else_help = true)]
        input: PathBuf,
        output: PathBuf,

        #[clap(flatten)]
        feature_group: Option<FeatureGroup>,

        #[clap(flatten)]
        trailers_group: Option<TrailersGroup>,
    },
}

fn main() -> ExitCode {
    let args = Cli::parse();

    let error = match args.command {
        Commands::Info { file, output_json } => {
            info::print_info(&file[..], output_json, args.global_opts.verbose)
        }
        Commands::Extract {
            input,
            output,
            feature_group,
            trailers_group,
        } => {
            let feature = match feature_group {
                Some(feature_group) => match feature_group.feature_name {
                    Some(name) => Some(Feature::Name(FeatureName { name })),
                    None => match feature_group.feature_id {
                        Some(id) => Some(Feature::Id(FeatureId { id })),
                        None => None,
                    },
                },
                None => None,
            };
            let trailers = match trailers_group {
                Some(trailers_group) => match trailers_group.trailer_names {
                    Some(names) => Some(Trailers::Names(TrailerNames { names })),
                    None => match trailers_group.trailer_ids {
                        Some(ids) => Some(Trailers::Ids(TrailerIds { ids })),
                        None => None,
                    },
                },
                None => None,
            };
            extract::extract_files(input, output, feature, trailers, args.global_opts.verbose)
        }
    };
    match error {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            println!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

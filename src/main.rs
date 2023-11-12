mod directories;
mod filters;
mod slugify;
mod template_data;

use askama::Template;
use eyre::{bail, Result, WrapErr};
use rust_decimal::Decimal;
use std::io::BufRead;
use template_data::DaneFaktury;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PapturaError {
    #[error("Argument for value {0} was invalid")]
    BadArgumentFormat(String),
    #[error("Dodaj przynajmniej jeden przedmiot sprzedaży")]
    PrzedmiotSprzedazyMissing,
    #[error("Faktura o takiej nazwie już istnieje, prawdopodobnie coś poszło nietak")]
    DocumentAlreadyExists,
}

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Wystawianie faktur bez badziewia
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// określa czy config ma byc pobrany z STDIN
    #[arg(long)]
    stdin: bool,
    /// ścieżka do pliku YAML z configiem dla danego klienta (wyklucza się z
    /// --stdin)
    #[arg(long, value_name = "FILE")]
    config_path: Option<PathBuf>,
    /// dodatkowe opcje
    #[command(subcommand)]
    command: Option<Commands>,
    /// ścieżka do folderu w którym ma być zapisana faktura
    #[arg(short, long, value_name = "DIRECTORY")]
    output_directory: PathBuf,
    #[arg(long)]
    cena_netto: Option<rust_decimal::Decimal>,
    #[arg(long)]
    zaplacono: Option<rust_decimal::Decimal>,
    #[arg(long)]
    extra_comment: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// wypisuje przykładowy config w YAMLu
    ExampleConfig,
}

fn main() -> Result<()> {
    color_eyre::install().ok();
    // let matches: clap::ArgMatches = App::new("paptura")
    //     .version(crate_version!())
    //     .author("Wojciech Niedźwiedź. <wojciech.brozek@niedzwiedz.it>")
    //     .about("Wystawianie faktur bez badziewia")
    //     .arg(
    //         Arg::with_name("example-config")
    //             .short("e")
    //             .long("example-config")
    //             .help("wypisuje przykładowy config w YAMLu"),
    //     )
    //     .arg(
    //         Arg::with_name("cena-netto")
    //             .short("c")
    //             .long("cena-netto")
    //             .value_name("PIENIĄDZE")
    //             .help("nadpisuje cenę netto towaru / usługi, np '2137.99'"),
    //     )
    //     .arg(
    //         Arg::with_name("stdin")
    //             .short("s")
    //             .long("stdin")
    //             .help("określa czy config ma byc pobrany z STDIN"),
    //     )
    //     .arg(
    //         Arg::with_name("config-path")
    //             .short("p")
    //             .long("config-path")
    //             .value_name("FILE")
    //             .help(
    //                 "ścieżka do pliku YAML z configiem dla danego klienta
    // (wyklucza się z --stdin)",             ),
    //     )
    //     .arg(
    //         Arg::with_name("output-directory")
    //             .short("o")
    //             .long("output-directory")
    //             .value_name("DIR")
    //             .help("ścieżka do folderu w którym ma być zapisana faktura"),
    //     )
    //     .get_matches();
    let Cli {
        stdin,
        config_path,
        command,
        output_directory,
        cena_netto,
        zaplacono,
        extra_comment,
    } = Cli::parse();
    match command {
        Some(subcommand) => match subcommand {
            Commands::ExampleConfig => {
                println!(
                    "{}",
                    serde_yaml::to_string(&DaneFaktury::default())
                        .wrap_err("formatting yaml to string")?
                );
                Ok(())
            }
        },
        None => {
            let config_str = {
                match (config_path, stdin) {
                    (Some(config_path), _) => std::fs::read_to_string(&config_path)
                        .wrap_err_with(|| format!("reading {config_path:?}"))?,
                    (_, true) => std::io::stdin()
                        .lock()
                        .lines()
                        .map_while(Result::ok)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => {
                        bail!("albo --stdin albo --config-path jest wymagane")
                    }
                }
            };

            let mut dane_faktury: DaneFaktury = serde_json::from_str(config_str.as_str())
                .or(serde_yaml::from_str(config_str.as_str()))
                .wrap_err("parsing config")?;
            // let output_directory =
            //     matches
            //         .value_of_os("output-directory")
            //         .ok_or(PapturaError::BadArgumentFormat(
            //             "output-directory is required".to_string(),
            //         ))?;
            // let output_directory = std::path::PathBuf::from(output_directory);
            if !output_directory.exists() || !output_directory.is_dir() {
                eyre::bail!("[{output_directory:?}] does not exits or is not a valid directory")
            }

            if let Some(cena_netto) = cena_netto {
                let first_entry = dane_faktury
                    .przedmiot_sprzedazy
                    .first_mut()
                    .ok_or(PapturaError::PrzedmiotSprzedazyMissing)
                    .wrap_err("Przedmiot Sprzedazy - required entry")?;
                first_entry.cena_netto = cena_netto;
                if let Some(zaplacono) = zaplacono {
                    let total_net: Decimal = dane_faktury
                        .przedmiot_sprzedazy
                        .iter()
                        .map(|p| p.cena_netto)
                        .sum();
                    if total_net < zaplacono {
                        bail!("zapłacono ({zaplacono}) więcej niż wynosi całkowita wartość faktury netto ({total_net})")
                    }
                    dane_faktury.zaplacono = zaplacono;
                }
            }

            if let Some(extra_comment) = extra_comment {
                dane_faktury.extra_comments = Some(extra_comment);
            }

            let slug = dane_faktury.slug();
            dane_faktury.numer_faktury = Some(
                dane_faktury.poczatek_serii_numeru_faktury
                    + std::fs::read_dir(&output_directory)
                        .wrap_err("reading [{output_directory:?}]")?
                        .filter_map(|entry| entry.ok())
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .filter(|name| name.starts_with(slug.as_str()))
                        .count() as u64,
            );
            let filename = dane_faktury.filename();
            let filepath = output_directory.join(filename);
            if filepath.exists() {
                eyre::bail!("[{filepath:?}] already exists!")
            }
            std::fs::write(
                &filepath,
                dane_faktury.render().wrap_err("rendering html document")?,
            )
            .wrap_err_with(|| format!("saving html document to [{filepath:?}]"))?;
            println!("{}", filepath.canonicalize()?.to_string_lossy());
            Ok(())
        }
    }
}

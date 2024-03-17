use anyhow::Context;
use clap::{Parser, Subcommand};
use std::ffi::CStr;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::{any, fs};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

/// Doc comment
#[derive(Debug, Subcommand)]
enum Command {
    /// Doc comment
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
}

enum Kind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main").unwrap();
            println!(
                "Initialized empty Git repository in {}",
                std::env::current_dir().unwrap().display()
            );
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            anyhow::ensure!(
                pretty_print,
                "only pretty-printing is supported at the moment"
            );
            let mut f = fs::File::open(
                Path::new(".git/objects")
                    .join(&object_hash[..2])
                    .join(&object_hash[2..]),
            )
            .context("open in .git/objects")?;
            let z = flate2::read::ZlibDecoder::new(&mut f);
            let mut z = std::io::BufReader::new(z);
            let mut buf = Vec::new();
            let _ = z.read_until(0, &mut buf);
            let header = CStr::from_bytes_with_nul(&buf)
                .expect("know there is exactly one nul byte, and it is at the end of the header");
            let header = header
                .to_str()
                .context(".git/objects file header isn't valid UTF-8")?;
            let Some((kind, size)) = header.split_once(' ') else {
                anyhow::bail!(
                    ".git/objects file header is not valid. Did not start with 'blob ': '{header}'"
                );
            };
            let kind = match kind {
                "blob" => Kind::Blob,
                _ => anyhow::bail!("unsupported object kind: {kind}"),
            };
            let Some(size) = header
                .strip_prefix("blob ")
                .or_else(|| header.strip_prefix("tree "))
                .or_else(|| header.strip_prefix("commit "))
                .or_else(|| header.strip_prefix("tag "))
                .map(|s| {
                    s.parse::<usize>()
                        .expect(".git/objects file header has invalid size: {size}")
                })
            else {
                anyhow::bail!(
                    ".git/objects file header is not valid. Did not start with 'blob ': '{header}'"
                );
            };
            buf.clear();
            buf.resize(size, 0);
            z.read_exact(&mut buf)
                .context("read true contents of .git/objects file")?;
            let n = z
                .read(&mut [0])
                .context("validate EOF in .git/object file")?;
            anyhow::ensure!(n == 0, ".git/objects file had {n} extra bytes");
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            match kind {
                Kind::Blob => stdout.write_all(&buf).context("write to stdout")?,
            }
        }
    }

    Ok(())
}

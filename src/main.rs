use native_tls::TlsConnector;
use std::{
    env,
    error,
    fmt::{
        self,
        Display
    },
    fs,
    io::{
        self,
        BufRead,
        Read,
        Write
    },
    net::{
        TcpStream
    },
    path::PathBuf,
    time::Duration
};
use structopt::StructOpt;
use url::Url;


#[derive(Debug, StructOpt)]
#[structopt(
    name = "git-remote-gemini", 
    version = "0.1",
    author = "David Huseby <dwh@vi.rs>",
    about = "Git remote helper for cloning over Gemini",
)]
struct Opt {
    /// remote name
    remote: String,

    /// remote URL
    url: Url
}

struct Ref {
    object: String,
    name: String
}

impl Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.object, self.name)
    }
}

impl From<Vec<String>> for Ref {
    fn from(item: Vec<String>) -> Self {
        Ref {
            object: item[0].clone(),
            name: item[1].clone()
        }
    }
}

pub fn get_data(url: &Url) -> Result<(Option<Vec<u8>>, Vec<u8>), Box<dyn error::Error>> {
    let host = url.host_str().unwrap();

    let mut builder = TlsConnector::builder();
    builder.danger_accept_invalid_hostnames(true);
    builder.danger_accept_invalid_certs(true);

    /*
    if let Some(cert) = crate::gemini::certificate::get_certificate(host) {
        let der = cert.to_der().unwrap();
        let identity = native_tls::Identity::from_pkcs12(&der, "").unwrap();
        builder.identity(identity);
    };
    */

    let connector = builder.build().unwrap();

    if let Ok(addrs) = url.socket_addrs(|| Some(1965 as u16)) {
        if let Ok(stream) = TcpStream::connect_timeout(&addrs[0], Duration::new(5, 0)) {
            if let Ok(mut mstream) = connector.connect(&host, stream) {
                let url = format!("{}\r\n", url);
                mstream.write_all(url.as_bytes()).unwrap();
                let mut res = Vec::<u8>::new();
                mstream.read_to_end(&mut res).unwrap();
                let clrf_idx = find_clrf(&res);
                let content = res.split_off(clrf_idx.unwrap() + 2);
                return Ok((Some(res), content));
            }
        }
    }
    return Err(From::from("failed to get content"));
}

fn find_clrf(data: &[u8]) -> Option<usize> {
    let clrf = b"\r\n";
    data.windows(clrf.len()).position(|window| window == clrf)
}

/*
fn touch(path: &PathBuf) -> Result<(), io::Error> {
    let mut stderr = std::io::stderr();
    match fs::OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => {
            writeln!(&mut stderr, "[GRG] touch: {}", path.to_str().unwrap())?;
            Ok(())
        },
        Err(e) => {
            writeln!(&mut stderr, "[GRG] touch err: {}", e)?;
            Err(e)
        }
    }
}
*/

fn main() -> Result<(), Box<dyn error::Error>> {
    let mut stderr = io::stderr();

    // parse the command line args
    let opt = Opt::from_args();
    /*
    writeln!(&mut stderr, "[GRG] remote: {}", opt.remote)?;
    writeln!(&mut stderr, "[GRG] scheme: {}", opt.url.scheme())?;
    writeln!(&mut stderr, "[GRG] path: {}", opt.url.path())?;
    */

    // grab the GIT_DIR set by Git
    let git_dir = env::var("GIT_DIR")?;
    //writeln!(&mut stderr, "[GRG] GIT_DIR: {}", git_dir)?;

    // set localdir
    let local_dir = {
        let mut local_dir = PathBuf::from(git_dir).canonicalize()?;
        local_dir.push("gemini");
        local_dir
    };
    fs::create_dir_all(local_dir.to_path_buf())?;
    //writeln!(&mut stderr, "[GRG] local_dir: {}", local_dir.to_str().unwrap())?;

    // make sure the marks files exist
    /*
    let gitmarks = local_dir.join("git.marks");
    let geminimarks = local_dir.join("gemini.marks");
    touch(&gitmarks)?;
    touch(&geminimarks)?;
    */

    // make sure that we have a scratch space for this remote
    /*
    let scratch_dir = local_dir.join(&opt.remote);
    fs::create_dir_all(scratch_dir.to_path_buf())?;
    writeln!(&mut stderr, "[GRG] scratch_dir: {}", scratch_dir.to_str().unwrap())?;
    */

    // set our refspec
    //let refspec = format!("refs/heads/*:refs/gemini/{}/*", opt.remote);
    //writeln!(&mut stderr, "[GRG] refspec: {}", refspec)?;

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let mut tokens: Vec<&str> = line.as_str().split(" ").collect();
        //writeln!(&mut stderr, "[GRG] => {}", line)?;
        match tokens[0] {
            "capabilities" => {
                /*
                writeln!(&mut stderr, "[GRG] <= fetch")?;
                println!("fetch");
                writeln!(&mut stderr, "[GRG] <= import")?;
                */
                println!("import");
                /*
                writeln!(&mut stderr, "[GRG] <= refspec {}", refspec)?;
                println!("refspec {}", refspec);
                writeln!(&mut stderr, "[GRG] <= \\n")?;
                */
                println!("");
            },
            "list" => {
                let mut url = opt.url.clone();
                url.set_query(Some("list"));
                if let Ok((_, content)) = get_data(&url) {
                    let mut stdout = io::stdout();
                    stdout.write_all(&content)?;
                    /*
                    writeln!(&mut stderr, "[GRG] <=")?;
                    stderr.write_all(&content)?;
                    writeln!(&mut stderr, "[GRG] <= {} bytes", content.len())?;
                    */
                }
            },
            "import" => {
                // for each ref make gemini request to gemini://<url>?fast-export=<ref>
                for r in tokens.drain(1..) {
                    let mut url = opt.url.clone();
                    let query = format!("fast-export={}", r);
                    url.set_query(Some(query.as_str()));
                    if let Ok((_, content)) = get_data(&url) {
                        let mut stdout = io::stdout();
                        stdout.write_all(&content)?;
                        writeln!(&mut stderr, "remote: importing {} bytes", content.len())?;
                        //writeln!(&mut stderr, "[GRG] <= {} bytes", content.len())?;
                    }
                }
            },
            "" => { 
                break; 
            },
            _ => {
                //writeln!(&mut stderr, "[GRG] unknown command: {}", line)?;
                return Err(From::from("unknown command from git"));
            }
        }
    }

    Ok(())
}

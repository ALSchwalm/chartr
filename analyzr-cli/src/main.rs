use analyzr_core::{event, load, render};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// What mode to run the program in
    #[command(subcommand)]
    mode: Command,

    path: PathBuf,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    Create(CreateArgs),
    AddActor(AddActorArgs),
    AddEvent(AddEventArgs),
}

#[derive(Args, Clone, Debug)]
struct CreateArgs {
    #[arg(long)]
    heading: Option<String>,
}

#[derive(Args, Clone, Debug)]
struct AddActorArgs {
    identity: String,
}

#[derive(Args, Clone, Debug)]
struct AddEventArgs {
    actor: String,
    start: i64,
    duration: Option<u32>,

    #[arg(short, long, default_value = "false")]
    endless: bool,

    #[arg(short, long)]
    color: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.mode {
        Command::Create(args) => {
            let mut builder = render::RendererBuilder::default();

            if let Some(heading) = args.heading {
                builder = builder.heading(heading)
            }

            let renderer = builder.build();
            let store = event::EventStore::default();
            renderer.render(cli.path, store).unwrap();
        }
        Command::AddActor(args) => {
            let (r, mut events) = load(&cli.path).unwrap();
            events
                .register_actor(event::Actor::new(args.identity))
                .unwrap();
            r.render(cli.path, events).unwrap();
        }
        Command::AddEvent(args) => {
            let (r, mut events) = load(&cli.path).unwrap();

            let kind = match args.duration {
                Some(duration) => event::EventKind::Span(args.start, Some(duration)),
                None => {
                    if args.endless {
                        event::EventKind::Span(args.start, None)
                    } else {
                        event::EventKind::Instant(args.start)
                    }
                }
            };

            let mut fields = std::collections::BTreeMap::default();

            if let Some(color) = args.color {
                fields.insert("fill".into(), color);
            }

            let e = event::Event {
                fields,
                value: "".into(),
                kind,
            };

            events.add_event(&args.actor, e).unwrap();
            r.render(cli.path, events).unwrap();
        }
    }
}

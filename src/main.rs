use anyhow::bail;
use cli_table::{format::Justify, print_stdout, Cell, Style, Table};
use dirs::home_dir;
use seahorse::{App, Command, Context};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage("todo [sub command] [args]")
        .command(ls_command())
        .command(add_command())
        .command(delete_command())
        .command(done_command())
        .command(clear_command())
        .action(ls_action);

    app.run(args);
}

fn ls_action(_: &Context) {
    let todos = Todos::read();
    todos.print_list();
}

fn ls_command() -> Command {
    Command::new("list")
        .description("Show all TODOs")
        .usage("todo list")
        .alias("ls")
        .alias("l")
        .action(ls_action)
}

fn add_command() -> Command {
    Command::new("add")
        .description("Add a TODO")
        .usage("todo add <text>")
        .alias("a")
        .action(|c| {
            let title = if !c.args.is_empty() {
                c.args.join(" ")
            } else {
                eprintln!("Please enter a title");
                exit(1);
            };

            let mut todos = Todos::read();

            if todos.add(title).is_err() {
                eprintln!("Failed to add.");
                exit(1);
            }

            todos.save(false).unwrap();
        })
}

fn delete_command() -> Command {
    Command::new("delete")
        .description("Delete a TODO with a specified ID")
        .usage("todo delete <todo id>")
        .alias("del")
        .action(|c| {
            let id = if c.args.len() == 1 {
                &c.args[0]
            } else {
                eprintln!("Please specify one ID");
                exit(1);
            };

            let mut todos = Todos::read();

            if todos.delete(id.clone()).is_err() {
                eprintln!("The specified ID does not exist");
                exit(1);
            }

            todos.save(true).unwrap();
        })
}

fn done_command() -> Command {
    Command::new("done")
        .description("Complete the TODO for the specified ID")
        .usage("todo done <todo id>")
        .alias("d")
        .action(|c| {
            let id = if c.args.len() == 1 {
                &c.args[0]
            } else {
                eprintln!("Please specify one ID");
                exit(1);
            };

            let mut todos = Todos::read();

            if todos.done(id.clone()).is_err() {
                eprintln!("The specified ID does not exist");
                exit(1);
            }

            todos.save(false).unwrap();
        })
}

fn clear_command() -> Command {
    Command::new("clear")
        .description("Delete all TODOs")
        .usage("todo clear")
        .alias("cl")
        .action(|_| {
            let mut todos = Todos::read();
            if todos.clear().is_err() {
                eprintln!("Failed to delete.");
                exit(1);
            }
            todos.save(true).unwrap();
        })
}

#[derive(Debug, Clone)]
struct Todo {
    id: String,
    title: String,
    done: String,
}

impl Todo {
    pub fn new(id: String, title: String, done: String) -> Self {
        Self { id, title, done }
    }

    pub fn to_csv(&self) -> String {
        format!("{},{},{}", self.id, self.title, self.done)
    }
}

#[derive(Debug, Clone)]
struct Todos {
    headers: Vec<String>,
    records: Vec<Todo>,
}

impl Todos {
    pub fn read() -> Self {
        let file = Self::read_file(true, true, true, false);
        let buf = BufReader::new(file);
        let bufs: Vec<String> = buf.lines().map(|l| l.unwrap()).collect();
        let records = if bufs.len() > 1 {
            bufs[1..]
                .iter()
                .map(|b| {
                    let v: Vec<String> = b.split(',').into_iter().map(|r| r.to_string()).collect();
                    Todo::new(v[0].clone(), v[1].clone(), v[2].clone())
                })
                .collect::<Vec<Todo>>()
        } else {
            vec![]
        };

        let headers = match bufs.first() {
            Some(h) => h.split(',').map(|a| a.to_string()).collect(),
            None => vec!["id".to_string(), "title".to_string(), "done".to_string()],
        };
        Todos { headers, records }
    }

    pub fn save(&mut self, t: bool) -> anyhow::Result<()> {
        let mut file = Self::read_file(false, true, true, t);
        let s = format!("{}\n{}", self.headers.join(","), self.to_vec().join("\n"));
        file.write_all(s.as_bytes())?;
        file.flush()?;

        Ok(())
    }

    fn read_file(r: bool, w: bool, c: bool, t: bool) -> File {
        OpenOptions::new()
            .read(r)
            .write(w)
            .create(c)
            .truncate(t)
            .open(Self::todo_path())
            .unwrap()
    }

    fn todo_path() -> String {
        let home = home_dir().unwrap();
        let home = home.to_str().unwrap();
        format!("{}/.todo", home)
    }

    pub fn done(&mut self, id: String) -> anyhow::Result<()> {
        let mut record = self.records.iter_mut().find(|r| r.id == id).unwrap();
        record.done = "✓".to_string();
        self.print_list();
        Ok(())
    }

    pub fn delete(&mut self, id: String) -> anyhow::Result<()> {
        let index = self.records.iter_mut().position(|r| r.id == id);
        let index = match index {
            Some(i) => i,
            None => bail!("The specified ID does not exist"),
        };
        self.records.remove(index);
        self.print_list();
        Ok(())
    }

    pub fn add(&mut self, title: String) -> anyhow::Result<()> {
        let last_id = match self.records.last() {
            Some(l) => l.id.parse().unwrap(),
            None => 0,
        };
        let id = last_id + 1;
        self.records
            .push(Todo::new(id.to_string(), title, "".into()));
        self.print_list();
        Ok(())
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        self.records = vec![];
        self.print_list();
        Ok(())
    }

    pub fn print_list(&self) {
        let table = self
            .records
            .iter()
            .map(|r| {
                vec![
                    r.id.clone().cell().justify(Justify::Center),
                    r.title.clone().cell(),
                    r.done.clone().cell().justify(Justify::Center),
                ]
            })
            .table()
            .title(
                self.headers
                    .iter()
                    .map(|h| h.to_uppercase().cell().bold(true).justify(Justify::Center)),
            )
            .bold(true);
        print_stdout(table).unwrap();
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.records.iter().map(|r| r.to_csv()).collect()
    }
}

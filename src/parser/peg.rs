use flow_control::Statement;

use std::process::Command;

use self::grammar::parse_;

use glob::glob;


#[derive(Debug, PartialEq, Clone)]
pub struct Redirection {
    pub file: String,
    pub append: bool
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pipeline {
    pub jobs: Vec<Job>,
    pub stdout: Option<Redirection>,
    pub stdin: Option<Redirection>,
}

impl Pipeline {
    pub fn new(jobs: Vec<Job>, stdin: Option<Redirection>, stdout: Option<Redirection>) -> Self {
        Pipeline {
            jobs: jobs,
            stdin: stdin,
            stdout: stdout,
        }
    }

    pub fn expand_globs(&mut self) {
        let jobs = self.jobs.drain(..).map(|mut job| {
            job.expand_globs();
            job
        }).collect();
        self.jobs = jobs;
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum JobKind { And, Background, Or, Pipe }

#[derive(Debug, PartialEq, Clone)]
pub struct Job {
    pub command: String,
    pub args: Vec<String>,
    pub kind: JobKind,
}

impl Job {
    pub fn new(args: Vec<String>, kind: JobKind) -> Self {
        let command = args[0].clone();
        Job {
            command: command,
            args: args,
            kind: kind,
        }
    }

    pub fn expand_globs(&mut self) {
        let mut new_args: Vec<String> = vec![];
        for arg in self.args.drain(..) {
            let mut pushed_glob = false;
            if arg.contains(|chr| chr == '?' || chr == '*' || chr == '[') {
                if let Ok(expanded) = glob(&arg) {
                    for path in expanded.filter_map(Result::ok) {
                        pushed_glob = true;
                        new_args.push(path.to_string_lossy().into_owned());
                    }
                }
            }
            if !pushed_glob {
                new_args.push(arg);
            }
        }
        self.args = new_args;
    }

    pub fn build_command(&self) -> Command {
        let mut command = Command::new(&self.command);
        for i in 1..self.args.len() {
            if let Some(arg) = self.args.get(i) {
                command.arg(arg);
            }
        }
        command
    }
}

pub fn parse(code: &str) -> Statement {
    match parse_(code) {
		Ok(code_ok) => code_ok,
		Err(err) => {
			println!("ion: Syntax {}",err);
			Statement::Pipelines(vec![])
		}
	}
}

peg_file! grammar("grammar.rustpeg");

#[cfg(test)]
mod tests {
    use super::grammar::*;
    use flow_control::{Statement, Comparitor};

    #[test]
    fn full_script() {
        pipelines(r#"if a == a
  echo true a == a

  if b != b
    echo true b != b
  else
    echo false b != b

    if 3 > 2
      echo true 3 > 2
    else
      echo false 3 > 2
    fi
  fi
else
  echo false a == a
fi
"#)
            .unwrap();  // Make sure it parses
    }

    #[test]
    fn leading_and_trailing_junk() {
        pipelines(r#"

# comment
   # comment


    if a == a
  echo true a == a  # Line ending commment

  if b != b
    echo true b != b
  else
    echo false b != b

    if 3 > 2
      echo true 3 > 2
    else
      echo false 3 > 2
    fi
  fi
else
  echo false a == a
      fi

# comment

"#).unwrap();  // Make sure it parses
    }
    #[test]
    fn parsing_ifs() {
        // Default case where spaced normally
        let parsed_if = if_("if 1 == 2").unwrap();
        let correct_parse = Statement::If{left: "1".to_string(),
                                        comparitor: Comparitor::Equal,
                                        right: "2".to_string()};
        assert_eq!(correct_parse, parsed_if);

        // Trailing spaces after final value
        let parsed_if = if_("if 1 == 2         ").unwrap();
        let correct_parse = Statement::If{left: "1".to_string(),
                                        comparitor: Comparitor::Equal,
                                        right: "2".to_string()};
        assert_eq!(correct_parse, parsed_if);

        // Default case where spaced normally
        let parsed_if = if_("if 1 <= 2").unwrap();
        let correct_parse = Statement::If{left: "1".to_string(),
                                        comparitor: Comparitor::LessThanOrEqual,
                                        right: "2".to_string()};
        assert_eq!(correct_parse, parsed_if);
    }

    #[test]
    fn parsing_elses() {
        // Default case where spaced normally
        let parsed_if = else_("else").unwrap();
        let correct_parse = Statement::Else;
        assert_eq!(correct_parse, parsed_if);

        // Trailing spaces after final value
        let parsed_if = else_("else         ").unwrap();
        let correct_parse = Statement::Else;
        assert_eq!(correct_parse, parsed_if);

        // Leading spaces after final value
        let parsed_if = else_("         else").unwrap();
        let correct_parse = Statement::Else;
        assert_eq!(correct_parse, parsed_if);
    }

    #[test]
    fn parsing_ends() {
        // Default case where spaced normally
        let parsed_if = end_("end").unwrap();
        let correct_parse = Statement::End;
        assert_eq!(correct_parse, parsed_if);

        // Trailing spaces after final value
        let parsed_if = end_("end         ").unwrap();
        let correct_parse = Statement::End;
        assert_eq!(correct_parse, parsed_if);

        // Leading spaces after final value
        let parsed_if = end_("         end").unwrap();
        let correct_parse = Statement::End;
        assert_eq!(correct_parse, parsed_if);
    }

    #[test]
    fn parsing_functions() {
        // Default case where spaced normally
        let parsed_if = fn_("fn bob").unwrap();
        let correct_parse = Statement::Function{name: "bob".to_string(), args: vec!()};
        assert_eq!(correct_parse, parsed_if);

        // Trailing spaces after final value
        let parsed_if = fn_("fn bob        ").unwrap();
        assert_eq!(correct_parse, parsed_if);

        // Leading spaces after final value
        let parsed_if = fn_("         fn bob").unwrap();
        assert_eq!(correct_parse, parsed_if);

        // Default case where spaced normally
        let parsed_if = fn_("fn bob a b").unwrap();
        let correct_parse = Statement::Function{name: "bob".to_string(), args: vec!("a".to_string(), "b".to_string())};
        assert_eq!(correct_parse, parsed_if);

        // Trailing spaces after final value
        let parsed_if = fn_("fn bob a b       ").unwrap();
        assert_eq!(correct_parse, parsed_if);

        // Leading spaces after final value
        let parsed_if = fn_("         fn bob a b").unwrap();
        assert_eq!(correct_parse, parsed_if);
    }
}

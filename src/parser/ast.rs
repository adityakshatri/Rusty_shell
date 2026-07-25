#[derive(Debug,Clone)]
pub enum RedirectMode {

    Input,

    Output,

    Append,

   // Error,

  //  ErrorAppend,

}
#[derive(Debug,Clone)]
pub struct Redirect {

    pub mode: RedirectMode,

    pub target: String,  //filename

}

#[derive(Debug)]
pub struct SimpleCommand {
    pub command: String,
    pub args: Vec<String>,
    //pub background: bool,
    pub redirects: Vec<Redirect>,
}
//defines the nodes and edges in AST
#[derive(Debug)]
pub enum AST {
    Simple(SimpleCommand),
}


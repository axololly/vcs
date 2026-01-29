// use std::fmt::{Debug, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use libasc::{hash::ObjectHash, key::Signature, user::User};

// #[derive(Deserialize, Serialize)]
// pub enum Message {
//     HasCommit(ObjectHash),
//     WantsCommit(ObjectHash),
//     GivesCommit(Box<Snapshot>, Vec<ObjectHash>),

//     HasContent(ObjectHash),
//     WantsContent(ObjectHash),
//     GivesContent(Content, ObjectHash),

//     HasTag(String, ObjectHash),
//     WantsTag(String),
//     GivesTag(String, ObjectHash),

//     HasBranch(String, ObjectHash),
//     WantsBranch(String),
//     GivesBranch(String, ObjectHash),

//     Error(String)
// }

// impl Debug for Message {
//     fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
//         use Message::*;
    
//         match self {
//             HasCommit(hash) => write!(f, "HasCommit({hash:?})"),
//             WantsCommit(hash) => write!(f, "WantsCommit({hash:?})"),
//             GivesCommit(commit, parents) => write!(f, "GivesCommit({commit:?}, {parents:?})"),
            
//             HasContent(hash) => write!(f, "HasContent({hash:?})"),
//             WantsContent(hash) => write!(f, "WantsContent({hash:?})"),
//             GivesContent(content, hash) => match content {
//                 Content::Literal(_) => write!(f, "GivesContent(Literal(_), {hash:?})"),
//                 Content::Delta(_) => write!(f, "GivesContent(Delta(_, _), {hash:?})")
//             },

//             HasBranch(name, hash) => write!(f, "HasBranch({name:?}, {hash:?})"),
//             WantsBranch(name) => write!(f, "WantsBranch({name:?})"),
//             GivesBranch(name, hash) => write!(f, "GivesBranch({name:?}, {hash:?})"),

//             HasTag(name, hash) => write!(f, "HasTag({name:?}, {hash:?})"),
//             WantsTag(name) => write!(f, "WantsTag({name:?})"),
//             GivesTag(name, hash) => write!(f, "GivesTag({name:?}, {hash:?})"),

//             Error(message) => write!(f, "Error({message:?})")
//         }
//     }
// }

#[derive(Deserialize, Serialize)]
pub enum MethodType {
    Push,
    Pull,
    Clone
}

#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub project_code: ObjectHash,
    pub user: Signature,
    pub method: MethodType
}

#[derive(Deserialize, Serialize)]
pub struct ValidLoginReply {
    pub users: Vec<User>
}

pub type LoginResponse = Result<ValidLoginReply, String>;

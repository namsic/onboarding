#[derive(Debug)]
pub enum Operation {
    // Vote(true): Please vote for me
    // Vote(false): I vote for you
    Vote(bool),
    // Heartbeat: I am alive leader
    Heartbeat,
}

pub enum OperationResult {
    Ignore,
    Ok,
}

impl TryFrom<Vec<u8>> for Operation {
    type Error = ();

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let len = value.len();
        if len < 1 {
            return Err(());
        }
        match value[0] {
            0 /* Vote */=> {
                if len < 2 {
                    return Err(());
                }
                Ok(Self::Vote(value[1] != 0))
            }
            1 /* Heartbeat */ => Ok(Self::Heartbeat),
            _ => Err(()),
        }
    }
}

impl From<Operation> for Vec<u8> {
    fn from(value: Operation) -> Self {
        let mut ret = Self::new();
        match value {
            Operation::Vote(request) => {
                ret.push(0);
                ret.push(request as u8);
            }
            Operation::Heartbeat => ret.push(1),
        }
        ret
    }
}

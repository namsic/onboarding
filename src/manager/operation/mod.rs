use crate::manager::*;

#[derive(Debug)]
pub enum Operation {
    // Vote(Some(address)): Please vote for me
    // Vote(None): I vote for you
    Vote(Option<String>),
    // Heartbeat: I am alive leader
    Heartbeat,
}

pub enum OperationResult {
    Err,
    Ok,
    Broadcast(Operation),
    Response(Operation),
}

impl Operation {
    pub fn process(&self, cluster_manager: &mut ClusterManager) -> OperationResult {
        match self {
            Self::Vote(Some(_)) => OperationResult::Err,
            Self::Vote(None) => {
                if let State::Candidate(n) = cluster_manager.state {
                    if n < cluster_manager.cluster_map.len() as ClusterSize / 2 {
                        cluster_manager.set_state(State::Candidate(n + 1));
                    } else {
                        cluster_manager.set_state(State::Leader);
                    }
                    OperationResult::Ok
                } else {
                    OperationResult::Err
                }
            }
            _ => OperationResult::Ok,
        }
    }
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
                if len < 1 {
                    return Err(());
                }
                if len == 1 {
                    Ok(Self::Vote(None))
                } else {
                    let address = match std::str::from_utf8(&value[1..]) {
                        Ok(address) => address,
                        Err(e) => {
                            log::trace!("from_utf8({:?}): {}", &value[1..], e);
                            return Err(());
                        }
                    };
                    Ok(Self::Vote(Some(address.to_string())))
                }
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
                if let Some(address) = request {
                    ret.extend(address.as_bytes())
                }
            }
            Operation::Heartbeat => ret.push(1),
        }
        ret
    }
}

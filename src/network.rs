use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};
use std::collections::HashSet;
use std::future::ready;
use futures::stream::StreamExt;
use libp2p::{gossipsub, mdns, Multiaddr, noise, swarm::{NetworkBehaviour, SwarmEvent}, Swarm, tcp, yamux};
use libp2p::gossipsub::TopicHash;
use libp2p::kad::Behaviour;
use libp2p::swarm::THandler;
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;
use block::Block;
use crate::block;
use tokio::time;

#[derive(NetworkBehaviour)]
pub struct GossipNet {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct GossipSwarm {
    swarm: Swarm<GossipNet>,
    publishing_topic: Option<gossipsub::IdentTopic>,
}

impl GossipSwarm {
    pub fn new() -> Result<GossipSwarm, Box<dyn Error>>{
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_quic()
            .with_behaviour(|key| {
                // To content-address message, we can take the hash of message and use it as an ID.
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s);
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                // Set a custom gossipsub configuration
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1)) // This is set to aid debugging by not cluttering the log space
                    .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
                    // signing)
                    //.message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                    .build()
                    .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

                // build a gossipsub network behaviour
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )?;

                let mdns =
                    mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
                Ok(GossipNet { gossipsub, mdns })
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(10)))
            .build();
        Ok (GossipSwarm { swarm, publishing_topic: None, })
    }

    pub fn subscribe(&mut self, topic: gossipsub::IdentTopic) -> Result<(), Box<dyn Error>> {
        // Listen on all interfaces and whatever port the OS assigns
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/42070".parse()?)?;
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?; // Propagate errors if subscription fails
        Ok(())
    }

    pub fn publish(&mut self, topic: gossipsub::IdentTopic) -> Result<(),Box<dyn Error>> {
        self.publishing_topic = Some(topic); // propagates errors if fails
        Ok(())
    }

    pub async fn handle_events(&mut self) -> Result<Option<Block>, Box<dyn Error + Send>> {
        let timeout_duration = Duration::from_secs(5); // Set your desired timeout duration
        let timeout = time::timeout(timeout_duration, async {
            select! {
            event = self.swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(GossipNetEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                    Ok(None)
                },
                SwarmEvent::Behaviour(GossipNetEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered peer has expired: {peer_id}");
                        self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                    Ok(None)
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                    Ok(None)
                },
                SwarmEvent::Behaviour(GossipNetEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    println!("Peer that sent message: {}",peer_id);
                    let message_block = Block::from_json(&String::from_utf8(message.data).unwrap()).unwrap();
                    Ok(Some(message_block))
                },
                _ => Ok(None),
            }
        }
        });

        timeout.await.unwrap_or_else(|_| {
            println!("Timeout reached, no events received.");
            Ok(None) // Optionally return None or some fallback behavior after the timeout
        })
    }

    pub async fn send_message(&mut self, mut message: &mut tokio::sync::mpsc::Receiver<Block>) -> Result<(), Box<dyn Error>> {
        let msg = message.recv().await.unwrap();

        // Attempt to publish the block to the gossipsub network
        if let Err(e) = self.swarm
            .behaviour_mut().gossipsub
            .publish(self.publishing_topic.clone().unwrap(), msg.to_json()?)
        {
            return Err(Box::new(e));  // Return the error from gossipsub.publish
        }

        // searches for additional peers to prevent lockout of new peers
        match self.handle_events().await {
            Ok(Some(_)) => { } // if block is received, it is returned
            Ok(None) => {      // if no events are received, nothing is returned
            }
            Err(e) => {
                eprintln!("Error: {e}");  // Return the error from handle_blocks
            }
        }
        Ok(())
    }

}

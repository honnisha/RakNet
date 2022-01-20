use binary_utils::*;
use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use crate::{
    internal::queue::{Queue, SendPriority},
    protocol::{mcpe::motd::Motd, Packet, online::Disconnect},
    server::{RakEvent, RakNetVersion},
};

use crate::protocol::handler::{handle_offline, handle_online};

use super::state::ConnectionState;

pub type SendCommand = (String, Vec<u8>);

#[derive(Debug, Clone)]
pub struct Connection {
    /// The tokenized address of the connection.
    /// This is the identifier rak-rs will use to identify the connection.
    /// It follows the format `<ip>:<port>`.
    pub address: String,
    /// The current state of the connection.
    /// This is used to determine what packets can be sent and at what times.
    /// Some states are used internally to rak-rs, but are not used in actual protocol
    /// such as "Unidentified" and "Online".
    pub state: ConnectionState,
    /// The maximum transfer unit for the connection.
    /// Any outbound packets will be sharded into frames of this size.
    /// By default minecraft will use `1400` bytes. However raknet has 16 bytes of overhead.
    /// so this may be reduced as `1400 - 16` which is `1384`.
    pub mtu: u16,
    /// The last recieved time.
    /// This is used to determine if the connection has timed out.
    /// This is the time the last packet was recieved.
    pub recv_time: SystemTime,
    /// The time the server started.
    /// Used in pings
    pub start_time: SystemTime,
    /// The RakNet Version of the server.
    /// This is used to determine if the player can reliably join the server.
    pub raknet_version: RakNetVersion,
    /// Minecraft specific, the message of the day.
    pub motd: Motd,
    /// A reference to the server id.
    pub server_guid: u64,
    /// The packet queue for the connection.
    /// This is used to store packets that need to be sent, any packet here **WILL** be batched!
    pub queue: Queue<Vec<u8>>,
    /// This is an internal channel used on the raknet side to send packets to the user immediately.
    /// DO NOT USE THIS!
    pub send_channel: Arc<tokio::sync::mpsc::Sender<SendCommand>>,
    /// This is internal! This is used to dispatch events to the user.
    /// This will probably change in the near future, however this will stay,
    /// until that happens.
    pub event_dispatch: VecDeque<RakEvent>,
    /// This is internal! This is used to remove the connection if something goes wrong with connection states.
    /// (which is likely)
    ensure_disconnect: bool,
}

impl Connection {
    pub fn new(
        address: String,
        send_channel: Arc<tokio::sync::mpsc::Sender<SendCommand>>,
        start_time: SystemTime,
        server_guid: u64,
        port: String,
        raknet_version: RakNetVersion,
    ) -> Self {
        Self {
            address,
            state: ConnectionState::Unidentified,
            mtu: 1400,
            recv_time: SystemTime::now(),
            start_time,
            motd: Motd::new(server_guid, port),
            server_guid,
            queue: Queue::new(),
            send_channel,
            event_dispatch: VecDeque::new(),
            raknet_version,
            ensure_disconnect: false
        }
    }

    /// This method should be used externally to send packets to the connection.
    /// Packets here will be batched together and sent in frames.
    pub fn send_stream(&mut self, stream: Vec<u8>, priority: SendPriority) {
        if priority == SendPriority::Immediate {
            // todo: Create the frame and send it!
        } else {
            self.queue.push(stream, priority);
        }
    }

    /// This will send a raknet packet to the connection.
    /// This method will automatically parse the packet and send it by the given priority.
    pub fn send_packet(&mut self, packet: Packet, priority: SendPriority) {
        if priority == SendPriority::Immediate {
            self.send_immediate(packet.parse().unwrap());
        } else {
            self.queue
                .push(packet.parse().unwrap(), SendPriority::Normal);
        }
    }

    /// Adds the given stream to the connection's queue by priority.
    /// If instant is set to "true" the packet will be sent immediately.
    pub fn send(&mut self, stream: Vec<u8>, instant: bool) {
        if instant {
            // We're not going to batch this packet, so send it immediately.
            self.send_immediate(stream);
        } else {
            // We're going to batch this packet, so push it to the queue.
            self.queue.push(stream, SendPriority::Normal);
        }
    }

    /// Immediately send the packet to the connection.
    /// This will not automatically batch the packet.
    pub fn send_immediate(&mut self, stream: Vec<u8>) {
        if let Ok(_) =
            futures_executor::block_on(self.send_channel.send((self.address.clone(), stream)))
        {
            // GREAT!
        }
    }

    pub fn recv(&mut self, payload: &Vec<u8>) {
        self.recv_time = SystemTime::now();

        // let's verify our state.
        if !self.state.is_reliable() {
            // we got a packet when the client state was un-reliable, we're going to force the client
            // to un-identified.
            self.state = ConnectionState::Unidentified;
        }

        // build the packet
        if let Ok(packet) = Packet::compose(&payload, &mut 0) {
            // the packet is internal, let's check if it's an online packet or offline packet
            // and handle it accordingly.
            if packet.is_online() {
                // online packet
                // handle the connected packet
                handle_online(self, packet);
            } else {
                // offline packet
                // handle the disconnected packet
                handle_offline(self, packet);
            }
        } else {
            // this packet could be a Ack or Frame
            println!("We got a packet that we couldn't parse! Probably a Nak or Frame! Buffer: {:?}", payload);
        }
    }

    pub fn disconnect<S: Into<String>>(&mut self, reason: S, server_initiated: bool) {
        // disconnect!!!
        self.event_dispatch.push_back(RakEvent::Disconnect(self.address.clone(), reason.into()));
        // actually handle this internally, cause we can't send packets if we're disconnected.
        self.state = ConnectionState::Offline;
        // the following is a hack to make sure the connection is removed from the server.
        self.ensure_disconnect = true;
        // Freeze the queue, just in case this is a server sided disconnect.
        // Otherwise this is useless.
        self.queue.frozen = true;
        // We also need to flush the queue so packets aren't sent, because they are now useless.
        self.queue.flush();

        if server_initiated {
            self.send_packet(Disconnect {}.into(), SendPriority::Immediate);
        }
    }

    /// This reads an internal value! This may not be in relation to the client's CURRENT state!
    pub fn is_disconnected(&self) -> bool {
        return self.ensure_disconnect == true;
    }

    /// This is called every RakNet tick.
    /// This is used to update the connection state and send `Priority::Normal` packets.
    /// as well as other internal stuff like updating flushing Ack and Nack.
    pub fn tick(&mut self) {}
}

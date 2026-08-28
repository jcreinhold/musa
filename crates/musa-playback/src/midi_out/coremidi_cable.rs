//! The macOS ports: `CoreMIDI` virtual sources, or one chosen destination.
//!
//! Apple's words are used as Apple uses them. Musa **publishes sources**,
//! which a workstation *receives* from; an endpoint a workstation or an
//! interface already offers, which Musa *sends* to, is a **destination**.
//!
//! Nothing here decides anything. It is handed a batch of bytes and a port
//! index and puts one on the other. The `coremidi` types never leave this
//! file, which is what prompt 213's boundary asks for and what lets the rest
//! of the crate build on a machine that has no `CoreMIDI` at all.

use coremidi::{Client, Destination, Destinations, OutputPort, PacketBuffer, VirtualSource};

use crate::error::EngineError;
use crate::midi_out::cable::{Batch, Cable, Refused};
use crate::midi_out::sender::BATCH;
use crate::midi_out::{MidiEndpoint, MidiOutputConfig, MidiOutputTarget};

/// Bytes reserved for one window's packet list: a timestamp and a length
/// ahead of each three-byte message, with room for the list header. The
/// buffer is built once and only ever cleared, so a send never allocates.
const BUFFER_BYTES: usize = BATCH * 16 + 256;

/// Every destination the host offers.
pub(crate) fn endpoints() -> Vec<MidiEndpoint> {
    Destinations
        .into_iter()
        .filter_map(|destination| {
            let id = destination.unique_id()?;
            Some(MidiEndpoint {
                id: id.to_string(),
                name: destination.display_name().unwrap_or_else(|| id.to_string()),
            })
        })
        .collect()
}

/// Publish the ports one run needs.
pub(crate) fn open(config: &MidiOutputConfig, names: &[String]) -> Result<Box<dyn Cable>, EngineError> {
    let client = Client::new(&config.client).map_err(|status| {
        EngineError::MidiOutput(format!("CoreMIDI refused to open a client for Musa (status {status})"))
    })?;
    let ports = match &config.target {
        MidiOutputTarget::VirtualSources => {
            let mut sources = Vec::with_capacity(names.len());
            for name in names {
                let source = client.virtual_source(name).map_err(|status| {
                    EngineError::MidiOutput(format!(
                        "CoreMIDI refused to publish the source `{name}` (status {status})"
                    ))
                })?;
                sources.push(Port::Source(source));
            }
            sources
        }
        MidiOutputTarget::Destination(id) => {
            let destination = find(id).ok_or_else(|| EngineError::UnknownMidiDestination { id: id.clone() })?;
            let mut ports = Vec::with_capacity(names.len());
            for name in names {
                let port = client.output_port(name).map_err(|status| {
                    EngineError::MidiOutput(format!("CoreMIDI refused to open the port `{name}` (status {status})"))
                })?;
                ports.push(Port::Destination(port, destination.clone()));
            }
            ports
        }
    };
    let buffers = ports
        .iter()
        .map(|_| PacketBuffer::with_capacity(BUFFER_BYTES))
        .collect();
    // Each message is stamped "now" and handed over as it comes due: see
    // `super::TIMING` for why, and note that a report always says so.
    Ok(Box::new(CoreMidiCable {
        _client: client,
        ports,
        buffers,
    }))
}

fn find(id: &str) -> Option<Destination> {
    Destinations
        .into_iter()
        .find(|destination| destination.unique_id().is_some_and(|unique| unique.to_string() == id))
}

enum Port {
    Source(VirtualSource),
    Destination(OutputPort, Destination),
}

struct CoreMidiCable {
    /// Held because every endpoint it made dies with it.
    _client: Client,
    ports: Vec<Port>,
    buffers: Vec<PacketBuffer>,
}

impl Cable for CoreMidiCable {
    fn send(&mut self, port: usize, batch: &Batch) -> Result<(), Refused> {
        let (Some(buffer), Some(port)) = (self.buffers.get_mut(port), self.ports.get(port)) else {
            return Err(Refused);
        };
        buffer.clear();
        for (_, wire) in batch {
            buffer.push_data(0, wire.on_the_wire());
        }
        let sent = match port {
            Port::Source(source) => source.received(&*buffer),
            Port::Destination(output, destination) => output.send(destination, &*buffer),
        };
        sent.map_err(|_| Refused)
    }
}

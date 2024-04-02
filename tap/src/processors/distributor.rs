use std::{sync::{Arc, Mutex}, thread, process::exit};

use log::error;

use crate::{messagebus::bus::Bus, exit_code, data::tables::Tables, system_state::SystemState, metrics::Metrics};
use crate::processors::tcp_processor::TcpProcessor;

use super::{
    arp_processor::ARPProcessor,
    dns_processor::DnsProcessor,
    dot11_frame_processor::Dot11FrameProcessor,
};

pub fn spawn(ethernet_bus: Arc<Bus>, dot11_bus: Arc<Bus>, tables: &Arc<Tables>, system_state: Arc<SystemState>, metrics: Arc<Mutex<Metrics>>) {
    spawn_base_ethernet(ethernet_bus.clone());
    spawn_base_dot11(dot11_bus.clone(), &tables.clone());

    spawn_base_arp(ethernet_bus.clone(), tables.clone()); // TODO borrow
    
    spawn_base_tcp(ethernet_bus.clone(), &tables.clone());
    spawn_base_udp(ethernet_bus.clone(), &tables.clone());
    
    spawn_base_dns(ethernet_bus, tables, system_state, metrics);
}

// TODO don't exit here

fn spawn_base_ethernet(bus: Arc<Bus>) {
    thread::spawn(move || {
        for _packet in bus.ethernet_pipeline.receiver.iter() {
            // noop
        }

        error!("Ethernet receiver disconnected.");
        exit(exit_code::EX_UNAVAILABLE);
    });
}

fn spawn_base_dot11(bus: Arc<Bus>, tables: &Arc<Tables>) {
    let processor = Dot11FrameProcessor::new(tables.dot11.clone());

    thread::spawn(move || {
        for frame in bus.dot11_frames_pipeline.receiver.iter() {
            processor.process(&frame);
        }

        error!("802.11 frames receiver disconnected.");
        exit(exit_code::EX_UNAVAILABLE);
    });
}

fn spawn_base_arp(bus: Arc<Bus>, tables: Arc<Tables>) {
    let mut processor = ARPProcessor::new(tables);
    thread::spawn(move || {
        for packet in bus.arp_pipeline.receiver.iter() {
            processor.process(&packet);
        }

        error!("ARP receiver disconnected.");
        exit(exit_code::EX_UNAVAILABLE);
    });
}

fn spawn_base_tcp(bus: Arc<Bus>, tables: &Arc<Tables>) {
    let mut processor = TcpProcessor::new(tables.tcp.clone());
    
    thread::spawn(move || {
        for segment in bus.tcp_pipeline.receiver.iter() {
            processor.process(&segment);
        }

        error!("TCP receiver disconnected.");
        exit(exit_code::EX_UNAVAILABLE);
    });
}

fn spawn_base_udp(bus: Arc<Bus>, tables: &Arc<Tables>) {
    // TODO
}

fn spawn_base_dns(bus: Arc<Bus>, tables: &Arc<Tables>, system_state: Arc<SystemState>, metrics: Arc<Mutex<Metrics>>) {
    let mut processor = DnsProcessor::new(system_state, tables.dns.clone(), metrics);

    thread::spawn(move || {
        for packet in bus.dns_pipeline.receiver.iter() {
            processor.process(&packet);
        }

        error!("DNS receiver disconnected.");
        exit(exit_code::EX_UNAVAILABLE);
    });
}

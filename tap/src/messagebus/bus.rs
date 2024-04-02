use std::sync::{Arc, Mutex};

use crossbeam_channel::{Sender, Receiver, bounded};
use log::{debug, error};

use crate::{
    ethernet::packets::{
        EthernetPacket,
        ARPPacket,
        EthernetData,
        UDPPacket,
        DNSPacket,
        TcpSegment
    },
    metrics::Metrics, dot11::frames::{Dot11RawFrame, Dot11Frame}
};
use crate::configuration::Configuration;
use crate::messagebus::channel_names::{Dot11ChannelName, EthernetChannelName};

pub struct Bus {
    pub name: String,

    pub ethernet_broker: NzymeChannel<EthernetData>,
    pub dot11_broker: NzymeChannel<Dot11RawFrame>,

    pub dot11_frames_pipeline: NzymeChannel<Dot11Frame>,

    pub ethernet_pipeline: NzymeChannel<EthernetPacket>,
    pub arp_pipeline: NzymeChannel<ARPPacket>,
    pub tcp_pipeline: NzymeChannel<TcpSegment>,
    pub udp_pipeline: NzymeChannel<UDPPacket>,
    pub dns_pipeline: NzymeChannel<DNSPacket>
}

pub struct NzymeChannelSender<T> {
    sender: Sender<Arc<T>>,
    name: String,
    metrics: Arc<Mutex<Metrics>>
}

pub struct NzymeChannel<T> {
    pub sender: Mutex<NzymeChannelSender<T>>,
    pub receiver: Arc<Receiver<Arc<T>>>,
}

impl<T> NzymeChannelSender<T> {

    pub fn send_packet(&mut self, packet: Arc<T>, packet_length: u32) {
        if let Err(err) = self.sender.try_send(packet) {
            debug!("Could not write to channel [{:?}]: {}", self.name, err);
            
            match self.metrics.lock() {
                Ok(mut metrics) => {
                    metrics.increment_channel_errors(&self.name, 1);
                },
                Err(e) => { error!("Could not acquire metrics mutex: {}", e) }
            }
        } else {
            // Record metrics.
            match self.metrics.lock() {
                Ok(mut metrics) => {
                    metrics.record_channel_capacity(&self.name, self.sender.capacity().unwrap() as u128);
                    metrics.record_channel_watermark(&self.name, self.sender.len() as u128);
                    metrics.increment_channel_throughput_messages(&self.name, 1);
                    metrics.increment_channel_throughput_bytes(&self.name, packet_length);
                },
                Err(e) => { error!("Could not acquire metrics mutex: {}", e) }
            }
        }
    }

}

impl Bus<> {

    pub fn new(metrics: Arc<Mutex<Metrics>>, name: String, configuration: Configuration) -> Self {
        let (ethernet_broker_sender, ethernet_broker_receiver) = bounded(configuration.performance.ethernet_broker_buffer_capacity); // TODO configurable
        let (dot11_broker_sender, dot11_broker_receiver) = bounded(configuration.performance.wifi_broker_buffer_capacity);

        let (dot11_frames_sender, dot11_frames_receiver) = bounded(configuration.performance.wifi_broker_buffer_capacity);

        let (ethernet_pipeline_sender, ethernet_pipeline_receiver) = bounded(65536); // TODO configurable
        let (arp_pipeline_sender, arp_pipeline_receiver) = bounded(512); // TODO configurable

        let (tcp_pipeline_sender, tcp_pipeline_receiver) = bounded(configuration.protocols.tcp.pipeline_size as usize); // TODO configurable
        let (udp_pipeline_sender, udp_pipeline_receiver) = bounded(512); // TODO configurable

        let (dns_pipeline_sender, dns_pipeline_receiver) = bounded(512); // TODO configurable

        Self {
            name,
            ethernet_broker: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),
                    sender: ethernet_broker_sender,
                    name: EthernetChannelName::EthernetBroker.to_string()
                }),
                receiver: Arc::new(ethernet_broker_receiver),
            },
            dot11_broker: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),
                    sender: dot11_broker_sender,
                    name: Dot11ChannelName::Dot11Broker.to_string()
                }),
                receiver: Arc::new(dot11_broker_receiver),
            },
            dot11_frames_pipeline: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),
                    sender: dot11_frames_sender,
                    name: Dot11ChannelName::Dot11FramesPipeline.to_string()
                }),
                receiver: Arc::new(dot11_frames_receiver),
            },
            ethernet_pipeline: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),
                    sender: ethernet_pipeline_sender,
                    name: EthernetChannelName::EthernetPipeline.to_string()
                }),
                receiver: Arc::new(ethernet_pipeline_receiver),
            },
            arp_pipeline: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),  
                    sender: arp_pipeline_sender,
                    name: EthernetChannelName::ArpPipeline.to_string()
                }),
                receiver: Arc::new(arp_pipeline_receiver),
            },
            tcp_pipeline: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),
                    sender: tcp_pipeline_sender,
                    name: EthernetChannelName::TcpPipeline.to_string()
                }),
                receiver: Arc::new(tcp_pipeline_receiver),
            },
            udp_pipeline: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics: metrics.clone(),
                    sender: udp_pipeline_sender,
                    name: EthernetChannelName::UdpPipeline.to_string()
                }),
                receiver: Arc::new(udp_pipeline_receiver),
            },
            dns_pipeline: NzymeChannel {
                sender: Mutex::new(NzymeChannelSender {
                    metrics,
                    sender: dns_pipeline_sender,
                    name: EthernetChannelName::DnsPipeline.to_string()
                }),
                receiver: Arc::new(dns_pipeline_receiver),
            },
        }
    }

}

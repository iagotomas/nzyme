use std::{
    collections::HashMap,
    sync::{Arc, Mutex}, thread
};

use log::{error};
use std::time::Duration;
use crate::configuration::Configuration;
use crate::data::tcp_table::TcpTable;
use crate::link::leaderlink::Leaderlink;

use crate::metrics::Metrics;

use super::{dns_table::DnsTable, dot11_table::Dot11Table};

pub struct Tables {
    pub arp: Arc<Mutex<HashMap<String, HashMap<String, u128>>>>,
    pub dns: Arc<Mutex<DnsTable>>,
    pub dot11: Arc<Mutex<Dot11Table>>,
    pub tcp: Arc<Mutex<TcpTable>>
}

impl Tables {

    pub fn new(metrics: Arc<Mutex<Metrics>>, leaderlink: Arc<Mutex<Leaderlink>>, configuration: &Configuration) -> Self {
        Tables {
            arp: Arc::new(Mutex::new(HashMap::new())),
            dns: Arc::new(Mutex::new(DnsTable::new(metrics.clone(), leaderlink.clone()))),
            dot11: Arc::new(Mutex::new(Dot11Table::new(leaderlink.clone()))),
            tcp: Arc::new(Mutex::new(TcpTable::new(
                leaderlink,
                metrics,
                configuration.protocols.tcp.reassembly_buffer_size,
                configuration.protocols.tcp.session_timeout_seconds
            )))
        }
    }

    pub fn run_jobs(&self) {
        loop {
            thread::sleep(Duration::from_secs(10));

            match self.tcp.lock() {
                Ok(tcp) => {
                    tcp.calculate_metrics();
                    tcp.process_report();
                },
                Err(e) => error!("Could not acquire TCP table lock for report processing: {}", e)
            }

            match self.dot11.lock() {
                Ok(dot11) => dot11.process_report(),
                Err(e) => error!("Could not acquire 802.11 table lock for report processing: {}", e)
            }

            match self.dns.lock() {
                Ok(dns) => {
                    dns.calculate_metrics();
                    dns.process_report();
                },
                Err(e) => error!("Could not acquire DNS table lock for report processing: {}", e)
            }
        }
    }

}
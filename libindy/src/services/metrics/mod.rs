use crate::services::metrics::command_metrics::CommandMetric;
use convert_case::{Case, Casing};
use indy_api_types::errors::{IndyErrorKind, IndyResult, IndyResultExt};
use models::{MetricsValue, CommandCounters};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::HashMap;

pub mod command_metrics;
pub mod models;

const COMMANDS_COUNT: usize = MetricsService::commands_count();

pub struct MetricsService {
    queued_counters: RefCell<[CommandCounters; COMMANDS_COUNT]>,
    executed_counters: RefCell<[CommandCounters; COMMANDS_COUNT]>,
    callback_counters: RefCell<[CommandCounters; COMMANDS_COUNT]>,
}

impl MetricsService {
    pub fn new() -> Self {
        MetricsService {
            queued_counters: RefCell::new([CommandCounters::new(0,0,[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]); COMMANDS_COUNT]),
            executed_counters: RefCell::new([CommandCounters::new(0,0,[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]); COMMANDS_COUNT]),
            callback_counters: RefCell::new([CommandCounters::new(0,0,[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]); COMMANDS_COUNT]),
        }
    }

    pub fn cmd_left_queue(&self, command_index: CommandMetric, duration: u128) {
        self.queued_counters.borrow_mut()[command_index as usize].add(duration);
    }

    pub fn cmd_executed(&self, command_index: CommandMetric, duration: u128) {
        self.executed_counters.borrow_mut()[command_index as usize].add(duration);
    }

    pub fn cmd_callback(&self, command_index: CommandMetric, duration: u128) {
        self.callback_counters.borrow_mut()[command_index as usize].add(duration);
    }

    pub fn cmd_name(index: usize) -> String {
        CommandMetric::from(index).to_string().to_case(Case::Snake)
    }

    const fn commands_count() -> usize {
        CommandMetric::VARIANT_COUNT
    }

    pub fn get_command_tags(
        command: String,
        stage: String,
    ) -> HashMap<String, String> {
        let mut tags = HashMap::<String, String>::new();
        tags.insert("command".to_owned(), command.clone());
        tags.insert("stage".to_owned(), stage.to_owned());
        tags
    }

    pub fn append_command_metrics(&self, metrics_map: &mut Map<String, Value>) -> IndyResult<()> {
        let mut commands_count = Vec::new();
        let mut commands_duration_ms = Vec::new();
        let mut commands_duration_ms_bucket = Vec::new();

        for index in (0..MetricsService::commands_count()).rev() {
            let command = MetricsService::cmd_name(index);
            let tags_executed = MetricsService::get_command_tags(
                command.clone(),
                String::from("executed"),
            );
            let tags_queued = MetricsService::get_command_tags(
                command.clone(),
                String::from("queued"),
            );

            commands_count.push(
                serde_json::to_value(MetricsValue::new(
                    self.executed_counters.borrow()[index].count as usize,
                    tags_executed.clone(),
                ))
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
            );
            commands_count.push(
                serde_json::to_value(MetricsValue::new(
                    self.queued_counters.borrow()[index].count as usize,
                    tags_queued.clone(),
                ))
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
            );

            commands_duration_ms.push(
                serde_json::to_value(MetricsValue::new(
                    self.executed_counters.borrow()[index].duration_ms_sum as usize,
                    tags_executed.clone(),
                ))
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
            );
            commands_duration_ms.push(
                serde_json::to_value(MetricsValue::new(
                    self.queued_counters.borrow()[index].duration_ms_sum as usize,
                    tags_queued.clone(),
                ))
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
            );

            for index_bucket in (0..self.executed_counters.borrow()[index].duration_ms_bucket.len()).rev() {
                let executed_bucket = self.executed_counters.borrow()[index as usize].duration_ms_bucket[index_bucket];
                let queued_bucket = self.queued_counters.borrow()[index as usize].duration_ms_bucket[index_bucket as usize];

                commands_duration_ms_bucket.push(
                    serde_json::to_value(MetricsValue::new(
                        executed_bucket as usize,
                        tags_executed.clone(),
                    ))
                        .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
                );
                commands_duration_ms_bucket.push(
                    serde_json::to_value(MetricsValue::new(
                        queued_bucket as usize,
                        tags_queued.clone(),
                    ))
                        .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
                );

                commands_duration_ms_bucket.push(
                    serde_json::to_value(MetricsValue::new(
                        executed_bucket as usize,
                        tags_executed.clone(),
                    ))
                        .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
                );
                commands_duration_ms_bucket.push(
                    serde_json::to_value(MetricsValue::new(
                        queued_bucket as usize,
                        tags_queued.clone(),
                    ))
                        .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
                );
            }
        }

        metrics_map.insert(
            String::from("commands_count"),
            serde_json::to_value(commands_count)
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
        );
        metrics_map.insert(
            String::from("commands_duration_ms"),
            serde_json::to_value(commands_duration_ms)
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
        );
        metrics_map.insert(
            String::from("commands_duration_ms_bucket"),
            serde_json::to_value(commands_duration_ms_bucket)
                .to_indy(IndyErrorKind::IOError, "Unable to convert json")?,
        );

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_counters_are_initialized_as_zeros() {
        let metrics_service = MetricsService::new();
        for index in (0..MetricsService::commands_count()).rev() {
            assert_eq!(metrics_service.queued_counters.borrow()[index as usize].count, 0);
            assert_eq!(metrics_service.queued_counters.borrow()[index as usize].duration_ms_sum, 0);
            assert_eq!(metrics_service.executed_counters.borrow()[index as usize].count, 0);
            assert_eq!(metrics_service.executed_counters.borrow()[index as usize].duration_ms_sum, 0);
        }
    }

    #[test]
    fn test_cmd_left_queue_increments_relevant_queued_counters() {
        let metrics_service = MetricsService::new();
        let index = CommandMetric::IssuerCommandCreateSchema;
        let duration1 = 5u128;
        let duration2 = 2u128;

        metrics_service.cmd_left_queue(index, duration1);

        assert_eq!(metrics_service.queued_counters.borrow()[index as usize].count, 1);
        assert_eq!(metrics_service.queued_counters.borrow()[index as usize].duration_ms_sum, duration1);

        metrics_service.cmd_left_queue(index, duration2);

        assert_eq!(metrics_service.queued_counters.borrow()[index as usize].count, 1 + 1);
        assert_eq!(metrics_service.queued_counters.borrow()[index as usize].duration_ms_sum,
                   duration1 + duration2);
        assert_eq!(metrics_service.executed_counters.borrow()[index as usize].count, 0);
        assert_eq!(metrics_service.executed_counters.borrow()[index as usize].duration_ms_sum, 0);
    }

    #[test]
    fn test_cmd_executed_increments_relevant_executed_counters() {
        let metrics_service = MetricsService::new();
        let index = CommandMetric::IssuerCommandCreateSchema;
        let duration1 = 5u128;
        let duration2 = 2u128;

        metrics_service.cmd_executed(index, duration1);

        assert_eq!(metrics_service.executed_counters.borrow()[index as usize].count, 1);
        assert_eq!(metrics_service.executed_counters.borrow()[index as usize].duration_ms_sum, duration1);

        metrics_service.cmd_executed(index, duration2);

        assert_eq!(metrics_service.queued_counters.borrow()[index as usize].count, 0);
        assert_eq!(metrics_service.queued_counters.borrow()[index as usize].duration_ms_sum, 0);
        assert_eq!(metrics_service.executed_counters.borrow()[index as usize].count, 1 + 1);
        assert_eq!(metrics_service.executed_counters.borrow()[index as usize].duration_ms_sum, duration1 + duration2);
    }

    #[test]
    fn test_append_command_metrics() {
        let metrics_service = MetricsService::new();
        let mut metrics_map = serde_json::Map::new();

        metrics_service.append_command_metrics(&mut metrics_map);

        assert!(metrics_map.contains_key("commands_count"));
        assert!(metrics_map.contains_key("commands_duration_ms"));
        assert_eq!(
            metrics_map
                .get("commands_count")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            COMMANDS_COUNT * 2
        );
        assert_eq!(
            metrics_map
                .get("commands_duration_ms")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            COMMANDS_COUNT * 2
        );

        let commands_count = metrics_map
            .get("commands_count")
            .unwrap()
            .as_array()
            .unwrap();
        let commands_duration_ms = metrics_map
            .get("commands_duration_ms")
            .unwrap()
            .as_array()
            .unwrap();
        let commands_duration_ms_bucket = metrics_map
            .get("commands_duration_ms_bucket")
            .unwrap()
            .as_array()
            .unwrap();

        let expected_commands_count = [
            json!({"tags":{"command":"payments_command_build_set_txn_fees_req_ack","stage":"executed"},"value":0}),
            json!({"tags":{"command":"metrics_command_collect_metrics","stage":"queued"},"value":0}),
            json!({"tags":{"command":"cache_command_purge_cred_def_cache","stage":"executed"},"value":0}),
            json!({"tags":{"command": "non_secrets_command_fetch_search_next_records","stage":"queued"},"value":0}),
        ];

        let expected_commands_duration_ms = [
            json!({"tags":{"command":"payments_command_build_set_txn_fees_req_ack","stage":"executed"},"value":0}),
            json!({"tags":{"command":"metrics_command_collect_metrics","stage":"queued"},"value":0}),
            json!({"tags":{"command":"cache_command_purge_cred_def_cache","stage":"executed"},"value":0}),
            json!({"tags":{"command":"non_secrets_command_fetch_search_next_records","stage":"queued"},"value":0}),
        ];

        let expected_commands_duration_ms_bucket = [
            json!({"tags":{"command":"payments_command_build_set_txn_fees_req_ack","stage":"executed"},"value":0}),
            json!({"tags":{"command":"metrics_command_collect_metrics","stage":"queued"},"value":0}),
            json!({"tags":{"command":"cache_command_purge_cred_def_cache","stage":"executed"},"value":0}),
            json!({"tags":{"command":"non_secrets_command_fetch_search_next_records","stage":"queued"},"value":0}),
        ];

        for command in &expected_commands_count {
            assert!(commands_count.contains(&command));
        }

        for command in &expected_commands_duration_ms {
            assert!(commands_duration_ms.contains(&command));
        }

        for command in &expected_commands_duration_ms_bucket {
            assert!(commands_duration_ms_bucket.contains(&command));
        }
    }
}
// Copyright 2023 Google LLC

// TODO: Move this to a library crate

use har_grpc_services::vehicledata::*;
use log::warn;
use std::collections::HashMap;

// Maps from SDV topic name to HAR topic name
pub trait TopicMapper {
    /// Convert an SDV topic name to HAR topic name
    fn map_topic(&self, topic: String) -> String;
}

/// Maps SDV topic-message pairs into VehicleDataFragments
pub struct SdvToHarMapper<Mapper: TopicMapper> {
    mapper: Mapper,
}

impl Default for HashMapTopicMapper {
    fn default() -> Self {
        Self::new()
    }
}

// Allow unused functions, they might be used later.
// TODO: Remove allow usused after making this a public API in a lib.
#[allow(unused)]
impl<Mapper: TopicMapper> SdvToHarMapper<Mapper> {
    pub fn new(mapper: Mapper) -> Self {
        Self { mapper }
    }

    pub fn map_bool(&self, topic: String, message: bool) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointBool::new();
        data.dataBool = message;
        result.set_dataBool(data);
        result
    }

    pub fn map_i32(&self, topic: String, message: i32) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointI32::new();
        data.dataI32 = message;
        result.set_dataI32(data);
        result
    }

    pub fn map_u32(&self, topic: String, message: u32) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointU32::new();
        data.dataU32 = message;
        result.set_dataU32(data);
        result
    }

    pub fn map_f32(&self, topic: String, message: f32) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointF32::new();
        data.dataF32 = message;
        result.set_dataF32(data);
        result
    }

    pub fn map_f64(&self, topic: String, message: f64) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointF64::new();
        data.dataF64 = message;
        result.set_dataF64(data);
        result
    }

    pub fn map_u8(&self, topic: String, message: u8) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointU8::new();
        data.dataU8 = message as _;
        result.set_dataU8(data);
        result
    }

    pub fn map_string(&self, topic: String, message: String) -> VehicleDataFragment {
        let mut result = VehicleDataFragment::new();
        result.name = self.mapper.map_topic(topic);

        let mut data = VehicleDataPointString::new();
        data.dataString = message;
        result.set_dataString(data);
        result
    }
}

/// A simple HashMap based topic mapper implementation
pub struct HashMapTopicMapper {
    map: HashMap<String, String>,
}

impl HashMapTopicMapper {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    /// Maps `from` topic `to` topic name.
    pub fn add<T: Into<String>, U: Into<String>>(&mut self, from: T, to: U) {
        self.map.insert(from.into(), to.into());
    }
}

impl TopicMapper for HashMapTopicMapper {
    fn map_topic(&self, topic: String) -> String {
        self.map
            .get(&topic)
            .unwrap_or_else(|| {
                warn!("No mapping defined for {}", &topic);
                &topic
            })
            .clone()
    }
}

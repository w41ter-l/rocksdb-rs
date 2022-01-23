use crate::common::format::*;
use crate::common::{encode_var_uint32, get_var_uint32};
use bytes::{BufMut, BytesMut};

const WRITE_BATCH_HEADER: usize = 12;

pub struct WriteBatch {
    data: Vec<u8>,
    count: u32,
    flag: u32,
}

pub struct WriteBatchForDB {
    data: Vec<u8>,
    flag: u32,
    sequence: u64,
    count: u32,
}

pub enum WriteBatchItem<'a> {
    Put {
        cf: u32,
        key: &'a [u8],
        value: &'a [u8],
    },
    Delete {
        cf: u32,
        key: &'a [u8],
    },
}

impl WriteBatch {
    pub fn new() -> WriteBatch {
        WriteBatch {
            data: vec![0; WRITE_BATCH_HEADER],
            count: 0,
            flag: 0,
        }
    }

    pub fn clear(&mut self) {
        self.data.resize(WRITE_BATCH_HEADER, 0);
        self.count = 0;
        self.flag = 0;
    }

    pub fn put_cf(&mut self, cf: u32, key: &[u8], value: &[u8]) {
        let mut tmp: [u8; 5] = [0u8; 5];
        self.count += 1;
        if cf == 0 {
            self.data.push(ValueType::kTypeValue as u8);
        } else {
            self.data.push(ValueType::kTypeColumnFamilyValue as u8);
            let offset = encode_var_uint32(&mut tmp, cf);
            self.data.extend_from_slice(&tmp[..offset]);
        }
        let offset = encode_var_uint32(&mut tmp, key.len() as u32);
        self.data.extend_from_slice(&tmp[..offset]);
        self.data.extend_from_slice(key);
        let offset = encode_var_uint32(&mut tmp, value.len() as u32);
        self.data.extend_from_slice(&tmp[..offset]);
        self.data.extend_from_slice(value);
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        self.put_cf(0, key, value);
    }

    pub fn delete_cf(&mut self, cf: u32, key: &[u8]) {
        let mut tmp: [u8; 5] = [0u8; 5];
        self.count += 1;
        if cf == 0 {
            self.data.push(ValueType::kTypeDeletion as u8);
        } else {
            self.data.push(ValueType::kTypeColumnFamilyDeletion as u8);
            let offset = encode_var_uint32(&mut tmp, cf);
            self.data.extend_from_slice(&tmp[..offset]);
        }
        let offset = encode_var_uint32(&mut tmp, key.len() as u32);
        self.data.extend_from_slice(&tmp[..offset]);
        self.data.extend_from_slice(key);
    }

    pub fn delete(&mut self, key: &[u8]) {
        self.delete_cf(0, key);
    }

    fn set_count(&mut self, count: u32) {
        let c = count.to_le_bytes();
        self.data[8..].copy_from_slice(&c);
    }

    pub fn to_raw(&mut self) -> WriteBatchForDB {
        self.set_count(self.count);
        let data = std::mem::take(&mut self.data);
        WriteBatchForDB {
            data,
            flag: self.flag,
            sequence: 0,
            count: self.count,
        }
    }

    pub fn recycle(&mut self, batch: WriteBatchForDB) {
        self.data = batch.data;
    }
}

pub struct WriteBatchIter<'a> {
    batch: &'a WriteBatchForDB,
    sequence: u64,
    offset: usize,
}

impl WriteBatchForDB {
    pub fn iter(&self) -> WriteBatchIter<'_> {
        WriteBatchIter {
            batch: self,
            offset: 0,
            sequence: 0,
        }
    }

    pub fn set_sequence(&mut self, sequence: u64) {
        self.data[..8].copy_from_slice(&sequence.to_le_bytes());
        self.sequence = sequence;
    }

    pub fn count(&self) -> u32 {
        self.count
    }
}

impl<'a> WriteBatchIter<'a> {
    pub fn read_record(&mut self) -> Option<WriteBatchItem<'a>> {
        let tag = self.batch.data[self.offset];
        let mut cf = 0;
        self.offset += 1;
        if tag == ValueType::kTypeColumnFamilyValue as u8
            || tag == ValueType::kTypeColumnFamilyDeletion as u8
        {
            if let Some((l, cf_id)) = get_var_uint32(&self.batch.data[self.offset..]) {
                self.offset += l;
                cf = cf_id;
            } else {
                return None;
            }
        }
        if tag == ValueType::kTypeValue as u8 || tag == ValueType::kTypeColumnFamilyValue as u8 {
            if let Some((l, key_len)) = get_var_uint32(&self.batch.data[self.offset..]) {
                self.offset += l;
                let key_pos = self.offset;
                self.offset += key_len as usize;
                if self.offset > self.batch.data.len() {
                    return None;
                }
                let key = &self.batch.data[key_pos..self.offset];
                if let Some((l, value_len)) = get_var_uint32(&self.batch.data[self.offset..]) {
                    self.offset += l;
                    let v_pos = self.offset;
                    self.offset += value_len as usize;
                    if self.offset > self.batch.data.len() {
                        return None;
                    }
                    return Some(WriteBatchItem::Put {
                        cf,
                        key,
                        value: &self.batch.data[v_pos..self.offset],
                    });
                }
            }
            return None;
        } else if tag == ValueType::kTypeDeletion as u8
            || tag == ValueType::kTypeColumnFamilyDeletion as u8
        {
            if let Some((l, key_len)) = get_var_uint32(&self.batch.data[self.offset..]) {
                self.offset += l;
                let key_pos = self.offset;
                self.offset += key_len as usize;
                if self.offset > self.batch.data.len() {
                    return None;
                }
                let key = &self.batch.data[key_pos..self.offset];
                return Some(WriteBatchItem::Delete { cf, key });
            }

            return None;
        } else {
            None
        }
    }
}

impl<'a> Iterator for WriteBatchIter<'a> {
    type Item = WriteBatchItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_record()
    }
}
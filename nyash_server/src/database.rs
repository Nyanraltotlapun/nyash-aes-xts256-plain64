use std::{u64, u128};

use crate::num_utils;
use rand::{self, Rng};
use redb::{
    Database, MultimapTableDefinition, ReadableMultimapTable, ReadableTable, ReadableTableMetadata,
    TableDefinition,
};

//tables defenition
const DB_FILE: &str = "./data";
const RANGE_INFO: TableDefinition<u128, (u128, u128, u128)> = TableDefinition::new("range_info");

const RANGE_STATUS: MultimapTableDefinition<bool, u128> =
    MultimapTableDefinition::new("range_status");

const JOBS_TABLE: TableDefinition<u64, (u128, u128, u64, u64)> = TableDefinition::new("jobs_state");
const JOBS_FREE_IDS: TableDefinition<u64, ()> = TableDefinition::new("jobs_free_ids");

// -- main_range --
// (start_tweak_key) (end_tweak) (committed) (progress)
// (u128)            (u128,       u128,       u128)
// (u32,u32,u32,u32,u32,u32,u32,u32)

// -- jobs_state --
//(job_id) (tweak_key, start_key, len, start_time)
// u64    (u128,      u128,      u64, u64)

// get time is seconds
fn get_timestump() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Copy, Clone, Debug)]
struct JobRecord {
    job_id: u64,
    tweak_key: u128,
    start_key: u128,
    len: u64,
    start_time: u64,
}

impl JobRecord {
    pub fn from_data(rk: u64, rv: (u128, u128, u64, u64)) -> Self {
        Self {
            job_id: rk,
            tweak_key: rv.0,
            start_key: rv.1,
            len: rv.2,
            start_time: rv.3,
        }
    }

    pub fn from_acces_guard(
        ag: (
            redb::AccessGuard<'_, u64>,
            redb::AccessGuard<'_, (u128, u128, u64, u64)>,
        ),
    ) -> Self {
        Self::from_data(ag.0.value(), ag.1.value())
    }

    pub fn from_option(
        op: Option<(
            redb::AccessGuard<'_, u64>,
            redb::AccessGuard<'_, (u128, u128, u64, u64)>,
        )>,
    ) -> Option<Self> {
        match op {
            Some(ag) => Some(Self::from_acces_guard(ag)),
            None => None,
        }
    }

    pub fn from_result(
        res: Result<
            (
                redb::AccessGuard<'_, u64>,
                redb::AccessGuard<'_, (u128, u128, u64, u64)>,
            ),
            redb::StorageError,
        >,
    ) -> Result<Self, redb::Error> {
        let ag_res = res?;
        Ok(Self::from_data(ag_res.0.value(), ag_res.1.value()))
    }

    pub fn get_value(&self) -> (u128, u128, u64, u64) {
        (self.tweak_key, self.start_key, self.len, self.start_time)
    }
}

#[derive(Copy, Clone, Debug)]
struct RangeRecord {
    start_tweak: u128,
    end_tweak: u128,
    committed: u128,
    in_progress: u128,
}

impl RangeRecord {
    pub fn create_job(&mut self, job_id: u64, job_len: u64) -> JobRecord {
        let job_rec = JobRecord {
            job_id: job_id,
            tweak_key: self.end_tweak,
            start_key: self.in_progress,
            len: job_len,
            start_time: get_timestump(),
        };

        self.in_progress = self.in_progress.strict_add(job_len as u128); // panic if owerflow
        return job_rec;
    }

    pub fn new(start_tweak: u128) -> Self {
        Self {
            start_tweak: start_tweak,
            end_tweak: start_tweak,
            committed: 0,
            in_progress: 0,
        }
    }

    pub fn from_data(rk: u128, rv: (u128, u128, u128)) -> Self {
        Self {
            start_tweak: rk,
            end_tweak: rv.0,
            committed: rv.1,
            in_progress: rv.2,
        }
    }

    pub fn from_acces_guard(
        ag: (
            redb::AccessGuard<'_, u128>,
            redb::AccessGuard<'_, (u128, u128, u128)>,
        ),
    ) -> Self {
        Self::from_data(ag.0.value(), ag.1.value())
    }

    pub fn from_option(
        op: Option<(
            redb::AccessGuard<'_, u128>,
            redb::AccessGuard<'_, (u128, u128, u128)>,
        )>,
    ) -> Option<Self> {
        match op {
            Some(ag) => Some(Self::from_acces_guard(ag)),
            None => None,
        }
    }

    pub fn from_result(
        res: Result<
            (
                redb::AccessGuard<'_, u128>,
                redb::AccessGuard<'_, (u128, u128, u128)>,
            ),
            redb::StorageError,
        >,
    ) -> Result<Self, redb::Error> {
        let ag_res = res?;
        Ok(Self::from_data(ag_res.0.value(), ag_res.1.value()))
    }

    pub fn get_value(&self) -> (u128, u128, u128) {
        (self.end_tweak, self.committed, self.in_progress)
    }

    pub fn increase_progress(self, job_len: u64) -> Self {
        let (res, carry) = self.in_progress.carrying_add(job_len as u128, false);
        if carry == true {
            panic!("Increase progress resulted in u128 integer overflow, this should not happen!");
        }
        Self {
            in_progress: res,
            ..self
        }
    }
}

fn get_range(
    range_info: &mut redb::Table<'_, u128, (u128, u128, u128)>,
    range_status: &mut redb::MultimapTable<'_, bool, u128>,
) -> Result<Option<RangeRecord>, redb::Error> {
    // 1. Go tru active ranges and check if their in_progress less than u128::MAX
    let mut ranges_list: Vec<RangeRecord> = Vec::new();
    for wrap_id in range_status.get(false)? {
        let key = wrap_id?.value();
        let val = range_info
            .get(key)?
            .expect("range_table shuld contain key from range_active table")
            .value();

        let range = RangeRecord::from_data(key, val);

        if range.in_progress < u128::MAX {
            ranges_list.push(range);
        }
    }

    // If found active ranges to use, choose one randomly from them
    if ranges_list.len() > 0 {
        let r_idx: usize = rand::rng().random_range(0..ranges_list.len());
        return Ok(Some(ranges_list[r_idx]));
    }

    // Othervice, we need to create new range
    // If no ranges in table at all, we creating first one
    if range_info.len()? == 0 {
        let start_tweak = u128::MAX / 2;
        let new_range = RangeRecord::new(start_tweak);
        range_info.insert(start_tweak, new_range.get_value())?;
        range_status.insert(false, start_tweak)?;
        return Ok(Some(new_range));
    }

    //
    // if some records already exist, but no that we can use,
    // we should add new range
    let first_rec =
        RangeRecord::from_option(range_info.first()?).expect("Table should not be empty");
    let last_rec = RangeRecord::from_option(range_info.last()?).expect("Table should not be empty");

    let tweak_max = last_rec.end_tweak;
    let tweak_min = first_rec.start_tweak;

    // check if we are done!
    if tweak_max == u128::MAX && tweak_min == 0 {
        return Ok(None);
    }

    // randomly choosing to add new range to low or high part of the range
    let start_tweak = if rand::rng().random_bool(0.5) {
        // new range on upper
        if tweak_max < u128::MAX {
            tweak_max + 1
        } else {
            tweak_min - 1
        }
    } else {
        // new range on lower
        if tweak_min > 0 {
            tweak_min - 1
        } else {
            tweak_max + 1
        }
    };

    let new_range = RangeRecord::new(start_tweak);
    range_info.insert(start_tweak, new_range.get_value())?;
    range_status.insert(false, start_tweak)?;
    return Ok(Some(new_range));
}

// search for abandoned job
// returns first job older than 3 hours
fn find_abandoned_job(
    jobs_table: &redb::Table<'_, u64, (u128, u128, u64, u64)>,
) -> Result<Option<JobRecord>, redb::Error> {
    let current_time = get_timestump();
    for job_res in jobs_table.iter()? {
        let job_rec = JobRecord::from_result(job_res)?;
        // if job is older that 3 hours
        if (current_time - job_rec.start_time) > 10800 {
            return Ok(Some(job_rec));
        }
    }
    Ok(None)
}

// 1. Check if there is free ids to reuse
// If there is, returns first free ID
// and deletes it from jobs_free_ids_table
fn get_job_free_id(
    jobs_free_ids_table: &mut redb::Table<'_, u64, ()>,
) -> Result<Option<u64>, redb::Error> {
    // 1. Check if there is free ids to reuse
    if jobs_free_ids_table.len()? > 0 {
        match jobs_free_ids_table.pop_first()? {
            Some(ag) => Ok(Some(ag.0.value())),
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

// calling this if there was no free ids.
// What we want is to check max and min ids
// and return id lower than min or grater than max
fn get_new_job_id(
    jobs_table: &redb::Table<'_, u64, (u128, u128, u64, u64)>,
) -> Result<u64, redb::Error> {
    // If table is empty return 0 ID
    if jobs_table.len()? == 0 {
        return Ok(0);
    }

    // if min key greater than 0
    if let Some(ag) = jobs_table.first()? {
        let min_kv = ag.0.value();
        if min_kv > 0 {
            return Ok(min_kv.strict_sub(1u64));
        }
    } else {
        panic!("Because we checked for empty table, we should never get None here");
    }

    // now try to get new key from maximum key value
    if let Some(ag) = jobs_table.last()? {
        let max_kv = ag.0.value();
        if max_kv < u64::MAX {
            return Ok(max_kv.strict_add(1u64));
        } else {
            panic!("Key value should never reach u64 max value");
        }
    } else {
        panic!("Because we checked for empty table, we should never get None here");
    }
}

const MAX_U64_AS_U128: u128 = u64::MAX as u128;
fn db_get_job(db: Database, pref_job_size: u64) -> Result<Option<JobRecord>, redb::Error> {
    let transct = db.begin_write()?;

    // 1. look for abandonent jobs
    let new_job_opt = {
        let mut jobs_table = transct.open_table(JOBS_TABLE)?;

        if let Some(mut abandoned_job) = find_abandoned_job(&jobs_table)? {
            // update job start time
            abandoned_job.start_time = get_timestump();
            jobs_table.insert(abandoned_job.job_id, abandoned_job.get_value())?;
            Some(abandoned_job)
        } else {
            None
        }
    };

    if new_job_opt.is_some() {
        transct.commit()?;
        return Ok(new_job_opt);
    }

    // 2. look for free ids to reuse or generate new id
    let new_job_opt = {
        let mut range_info = transct.open_table(RANGE_INFO)?;
        let mut range_status = transct.open_multimap_table(RANGE_STATUS)?;
        let mut jobs_table = transct.open_table(JOBS_TABLE)?;
        let mut jobs_free_ids = transct.open_table(JOBS_FREE_IDS)?;

        // look for free ids to reuse
        let job_id_to_use = match get_job_free_id(&mut jobs_free_ids)? {
            Some(v) => v,
            None => {
                // need to generate ID
                get_new_job_id(&jobs_table)?
            }
        };

        // 3 Now get available range
        let range_opt = get_range(&mut range_info, &mut range_status)?;
        // if we found range - proceed. If None - means we already done.
        match range_opt {
            Some(mut range) => {
                let job_len =
                    pref_job_size.min(MAX_U64_AS_U128.min(u128::MAX - range.in_progress) as u64);
                if job_len == 0 {
                    panic!("Job len should not ended as 0");
                }
                let new_job = range.create_job(job_id_to_use, job_len);
                jobs_table.insert(new_job.job_id, new_job.get_value())?;
                range_info.insert(range.start_tweak, range.get_value())?;
                Some(new_job)
            }
            None => None, // We already done. All key space is processed.
        }
    };

    if new_job_opt.is_some() {
        // Commit transaction if Job was created
        transct.commit()?;
    }

    // If None - we already done. All key space is processed.
    // Return new_job_opt that contains Some if Job was created
    // or None if all ranges are searched
    return Ok(new_job_opt);

    // **********************************************
}

// fn test_db() -> Result<(), Error> {

//     let db = Database::create(DB_FILE)?;
//     let write_txn = db.begin_write()?;
//     {
//         let mut table = write_txn.open_table(RANGE_TABLE)?;
//         table.insert([0,1,2,3,0,124,6,7], (42u128, 42u128, false))?;

//     }
//     write_txn.commit()?;

//     let read_txn = db.begin_read()?;
//     let table = read_txn.open_table(RANGE_TABLE)?;
//    // println!("{}", table.get([0,1,2,3,3,5,6,7])?.unwrap().value());
//     let a = table.get([0,1,2,1,4,5,6,7]);
//     match a {
//         Ok(data) => match data {
//             Some(d) => println!("some data {}", d.value().0),
//             None => println!("None")
//         }
//         Err(_) => println!("Error")
//     }
//     assert_eq!(table.get([0,1,2,3,3,5,6,7])?.unwrap().value().0, 42u128);

//     Ok(())
// }

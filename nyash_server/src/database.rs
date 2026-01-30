use crate::num_utils;
use rand::{self, Rng, distr::weighted::Weight};
use redb::{
    Database, Key, MultimapTableDefinition, ReadableMultimapTable, ReadableTable, ReadableTableMetadata,
    TableDefinition,
};
// use std::collections::HashMap;
use std::{ops::Not, u64, u128};

//tables defenition
const DB_FILE: &str = "./data";

const COMPLETED_RANGES: TableDefinition<u128, u128> = TableDefinition::new("completed_ranges");

// const RANGE_STATUS: MultimapTableDefinition<bool, u128> =
//     MultimapTableDefinition::new("range_status");

const JOBS_TABLE: TableDefinition<u64, (u16, u128, u128, bool, u64, u64)> = TableDefinition::new("jobs");
const JOBS_FREE_IDS: TableDefinition<u64, ()> = TableDefinition::new("jobs_free_ids");

// get time is seconds
fn get_timestump() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[derive(Copy, Clone, Debug)]
struct JobRecord {
    id: u64,
    range_id: u16,
    tweak_key: u128,
    start_key: u128,
    on_range_tail: bool,
    len: u64,
    start_time: u64,
}

impl JobRecord {
    pub fn from_data(k: u64, v: &(u16, u128, u128, bool, u64, u64)) -> Self {
        Self {
            id: k,
            range_id: v.0,
            tweak_key: v.1,
            start_key: v.2,
            on_range_tail: v.3,
            len: v.4,
            start_time: v.5,
        }
    }

    pub fn from_iter_item(
        item: Result<
            (
                redb::AccessGuard<'_, u64>,
                redb::AccessGuard<'_, (u16, u128, u128, bool, u64, u64)>,
            ),
            redb::StorageError,
        >,
    ) -> Result<Self, redb::StorageError> {
        let ag = item?;
        Ok(Self::from_data(ag.0.value(), &ag.1.value()))
    }

    pub fn get_value(&self) -> (u16, u128, u128, bool, u64, u64) {
        return (
            self.range_id,
            self.tweak_key,
            self.start_key,
            self.on_range_tail,
            self.len,
            self.start_time,
        );
    }

    // search for abandoned job
    // returns first job older than 3 hours
    pub fn get_staled_job(
        jobs_table: &mut redb::Table<'_, u64, (u16, u128, u128, bool, u64, u64)>,
        timeout: u64,
    ) -> Result<Option<JobRecord>, redb::Error> {
        let current_time = get_timestump();
        let mut maybe_job: Option<JobRecord> = None;
        for job_res in jobs_table.iter()? {
            let job_rec = JobRecord::from_iter_item(job_res)?;
            // if job is older that timeout in seconds
            if (current_time - job_rec.start_time) > timeout {
                maybe_job = Some(job_rec);
                break;
            }
        }
        if let Some(mut job) = maybe_job {
            job.start_time = get_timestump();
            jobs_table.insert(job.id, job.get_value())?;
            return Ok(Some(job));
        }

        Ok(None)
    }

    // 1. Check if there is free ids to reuse
    // If there is, returns first free ID
    // and deletes it from jobs_free_ids_table
    fn pop_job_free_id(free_ids_t: &mut redb::Table<'_, u64, ()>) -> Result<Option<u64>, redb::Error> {
        // 1. Check if there is free ids to reuse
        Ok(free_ids_t.pop_first()?.and_then(|ag| Some(ag.0.value())))
    }

    // calling this if there was no free ids.
    // What we want is to check max and min ids
    // and return id lower than min or grater than max
    fn get_new_job_id(
        jobs_table: &redb::Table<'_, u64, (u16, u128, u128, bool, u64, u64)>,
    ) -> Result<u64, redb::Error> {
        // If table is empty return 0 ID
        if jobs_table.len()? == 0 {
            return Ok(0);
        }

        let first_id = jobs_table
            .first()?
            .expect("Because we checked for empty table, we should never get None here")
            .0
            .value();
        if first_id > 0 {
            return Ok(first_id.strict_sub(1u64));
        }

        let last_id = jobs_table
            .last()?
            .expect("Because we checked for empty table, we should never get None here")
            .0
            .value();
        if last_id < u64::MAX {
            return Ok(last_id.strict_add(1u64));
        }

        panic!("Key value should never reach u64 max value");
    }
}

const RANGES: TableDefinition<u16, (u128, u128, u128, u128, u128, u128)> = TableDefinition::new("ranges");
const RANGES_TO_USE: TableDefinition<u16, ()> = TableDefinition::new("ranges_to_use");
#[derive(Copy, Clone, Debug)]
struct RangeRecord {
    id: u16,
    tweak_head: u128,
    tweak_tail: u128,
    head_progress: u128,
    tail_progress: u128,
    head_in_progress: u128,
    tail_in_progress: u128,
}

impl RangeRecord {
    pub fn from_value(k: u16, v: &(u128, u128, u128, u128, u128, u128)) -> Self {
        Self {
            id: k,
            tweak_head: v.0,
            tweak_tail: v.1,
            head_progress: v.2,
            tail_progress: v.3,
            head_in_progress: v.4,
            tail_in_progress: v.5,
        }
    }

    pub fn create_job(&mut self, job_id: u64, pref_job_size: u64, on_tail: bool) -> JobRecord {
        const MAX_U64_AS_U128: u128 = u64::MAX as u128;

        let (tweak_key, start_key, use_tail, job_len) = if on_tail == true && self.tail_in_progress < u128::MAX {
            let start_key = self.tail_in_progress;
            let job_len = pref_job_size.min(MAX_U64_AS_U128.min(u128::MAX - start_key) as u64);
            self.tail_in_progress += job_len as u128;
            (self.tweak_tail, start_key, true, job_len)
        } else {
            let start_key = self.head_in_progress;
            let job_len = pref_job_size.min(MAX_U64_AS_U128.min(u128::MAX - start_key) as u64);
            self.head_in_progress += job_len as u128;
            (self.tweak_head, start_key, false, job_len)
        };

        if job_len == 0 {
            panic!("Job len should not ended as 0. We should never call create_job on busy range!");
        }

        JobRecord {
            id: job_id,
            range_id: self.id,
            tweak_key: tweak_key,
            start_key: start_key,
            on_range_tail: use_tail,
            len: job_len,
            start_time: get_timestump(),
        }
    }

    pub fn value(&self) -> (u128, u128, u128, u128, u128, u128) {
        (
            self.tweak_head,
            self.tweak_tail,
            self.head_progress,
            self.tail_progress,
            self.head_in_progress,
            self.tail_in_progress,
        )
    }
}

#[derive(Debug)]
struct RangesTable<'a> {
    ranges: redb::Table<'a, u16, (u128, u128, u128, u128, u128, u128)>,
    ranges_to_use: redb::Table<'a, u16, ()>,
}

impl<'a> RangesTable<'a> {
    pub fn new(
        ranges: redb::Table<'a, u16, (u128, u128, u128, u128, u128, u128)>,
        ranges_to_use: redb::Table<'a, u16, ()>,
    ) -> Self {
        RangesTable {
            ranges: ranges,
            ranges_to_use: ranges_to_use,
        }
    }

    pub fn open_rw(trx: &'a redb::WriteTransaction) -> Result<Self, redb::Error> {
        Ok(RangesTable {
            ranges: trx.open_table(RANGES)?,
            ranges_to_use: trx.open_table(RANGES_TO_USE)?,
        })
    }

    pub fn get_range(&self, k: u16) -> Result<Option<RangeRecord>, redb::Error> {
        Ok(self
            .ranges
            .get(k)?
            .and_then(|ag_v| Some(RangeRecord::from_value(k, &ag_v.value()))))
    }

    pub fn get_useful_range(&self) -> Result<Vec<RangeRecord>, redb::Error> {
        let mut res: Vec<RangeRecord> = Vec::with_capacity(self.ranges_to_use.len()? as usize);
        for kres in self.ranges_to_use.iter()? {
            res.push(
                self.get_range(kres?.0.value())?
                    .expect("Randge id from ranges_to_use should allways be present in ranges table"),
            );
        }

        Ok(res)
    }

    pub fn update_range(&mut self, range: &RangeRecord) -> Result<(), redb::Error> {
        self.ranges.insert(range.id, range.value())?;
        // if range head and tail reach max values - remove it from to_use table
        if range.head_in_progress == u128::MAX && range.tail_in_progress == u128::MAX {
            self.ranges_to_use.remove(range.id)?;
        }
        Ok(())
    }
}

fn db_create_job(db: Database, pref_job_size: u64) -> Result<Option<JobRecord>, redb::Error> {
    let transct = db.begin_write()?;

    // 1. look for abandonent jobs
    let new_job_opt = {
        let mut jobs_table = transct.open_table(JOBS_TABLE)?;
        JobRecord::get_staled_job(&mut jobs_table, 1200)?
    };

    if new_job_opt.is_some() {
        transct.commit()?;
        return Ok(new_job_opt);
    };

    // 2. look for free ids to reuse or generate new id
    let new_job_opt = {
        // let mut range_info = transct.open_table(RANGES)?;
        // let mut range_to_use = transct.open_table(RANGES_TO_USE)?;
        let mut jobs_table = transct.open_table(JOBS_TABLE)?;
        let mut jobs_free_ids = transct.open_table(JOBS_FREE_IDS)?;

        // look for free ids to reuse
        let job_id_to_use = match JobRecord::pop_job_free_id(&mut jobs_free_ids)? {
            Some(v) => v,
            None => {
                // need to generate ID
                JobRecord::get_new_job_id(&jobs_table)?
            }
        };

        // 3 Now get available range
        let mut ranges_table = RangesTable::open_rw(&transct)?;
        let ranges_to_choose = ranges_table.get_useful_range()?;
        if ranges_to_choose.len() == 0 {
            println!("No ranges to choose from!! This can happed if work is sone");
            return Ok(None);
        }

        // randomly choosing range from avaylable ranges
        let mut range_to_use = ranges_to_choose[rand::rng().random_range(0..ranges_to_choose.len())];

        // if we found range - proceed. If None - means we already done.
        let new_job = range_to_use.create_job(job_id_to_use, pref_job_size, rand::rng().random_bool(0.5));

        jobs_table.insert(new_job.id, new_job.get_value())?;
        ranges_table.update_range(&range_to_use)?;

        // finally!
        Some(new_job)
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

//fn commit_work()

fn db_commit_job(db: Database, job_id: u64) -> Result<bool, redb::Error> {
    let transct: redb::WriteTransaction = db.begin_write()?;

    {
        let mut jobs_table = transct.open_table(JOBS_TABLE)?;

        // 1. get job record
        let maybe_job_rec = jobs_table
            .get(job_id)?
            .and_then(|ag| Some(JobRecord::from_data(job_id, ag.value())));

        if let Some(job_rec) = maybe_job_rec {
            // 2. Remove job from jobs_table and add job_id to jobs_free_ids table
            jobs_table.remove(job_id)?;

            // add job_id to jobs_free_ids table
            let mut jobs_free_ids = transct.open_table(JOBS_FREE_IDS)?;
            jobs_free_ids.insert(job_id, ())?;

            let mut range_info = transct.open_table(RANGE_INFO)?;
            let job_range = RangeRecord::from_data(
                job_rec.tweak_key,
                range_info
                    .get(job_rec.tweak_key)?
                    .expect("Range to which Job reffering musbe present!")
                    .value(),
            );

            // commit job to range. Panik if overflow.
            job_range.committed.checked_add_assign(&(job_rec.len as u128)).unwrap();

            range_info.insert(job_rec.tweak_key, job_range.get_value())?;

            // check if Job is finished.
            if job_range.committed == u128::MAX {
                let mut range_status = transct.open_multimap_table(RANGE_STATUS)?;
                range_status.insert(true, job_range.start_tweak)?;
            }
        }

        let maybe_job_rec = match jobs_table.get(job_id)? {
            Some(ag) => JobRecord::from_data(job_id, ag.value()),
            None => None,
        };
    }

    return Ok(false); // Job already done by someone else
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

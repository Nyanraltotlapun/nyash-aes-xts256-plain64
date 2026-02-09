use crate::num_utils;
use rand::{self, Rng, distr::weighted::Weight};
use redb::{
    Database, Key, MultimapTableDefinition, ReadableDatabase, ReadableMultimapTable, ReadableTable, ReadableTableMetadata, TableDefinition
};
// use std::collections::HashMap;
use std::{ops::Not, u64, u128};

//tables defenition
const DB_FILE: &str = "./data";

const COMPLETED_RANGES: TableDefinition<u128, u128> = TableDefinition::new("completed_ranges");

// const RANGE_STATUS: MultimapTableDefinition<bool, u128> =
//     MultimapTableDefinition::new("range_status");

const JOBS_TABLE: TableDefinition<u64, (u16, u128, u128, u64, u64)> = TableDefinition::new("jobs");
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
    len: u64,
    start_time: u64,
}

impl JobRecord {
    pub fn from_data(k: u64, v: &(u16, u128, u128, u64, u64)) -> Self {
        Self {
            id: k,
            range_id: v.0,
            tweak_key: v.1,
            start_key: v.2,
            len: v.3,
            start_time: v.4,
        }
    }

    pub fn from_iter_item(
        item: Result<
            (
                redb::AccessGuard<'_, u64>,
                redb::AccessGuard<'_, (u16, u128, u128, u64, u64)>,
            ),
            redb::StorageError,
        >,
    ) -> Result<Self, redb::StorageError> {
        let ag = item?;
        Ok(Self::from_data(ag.0.value(), &ag.1.value()))
    }

    pub fn get_value(&self) -> (u16, u128, u128, u64, u64) {
        return (
            self.range_id,
            self.tweak_key,
            self.start_key,
            self.len,
            self.start_time,
        );
    }

    // search for abandoned job
    // returns first job older than 3 hours
    pub fn get_staled_job(
        jobs_table: &mut redb::Table<'_, u64, (u16, u128, u128, u64, u64)>,
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
        jobs_table: &redb::Table<'_, u64, (u16, u128, u128, u64, u64)>,
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

const RANGES: TableDefinition<u16, (u128, u128, u128, u128)> = TableDefinition::new("ranges");
const RANGES_TO_USE: TableDefinition<u16, ()> = TableDefinition::new("ranges_to_use");

// to not owerflow u128 we calculate it in range 0..max and then substruct
// const SUB_DIV: u128 = 0x1000100010001000100010001;
// TWEAKS_PER_RANGE: u128 = 5192296858534827628530496329220096
const TWEAKS_PER_RANGE: u128 = (u128::MAX / (u16::MAX as u128 + 1)) + 1;
const TWEAKS_PER_RANGE_F64: f64 = TWEAKS_PER_RANGE as f64;
//assert!(TWEAKS_PER_RANGE == 5192296858534827628530496329220096u128);

const TWEAKS_PER_RANGE_ID_VICE: u128 = TWEAKS_PER_RANGE - 1;
//const HALF_TWEAKS_PER_RANGE: u128 = TWEAKS_PER_RANGE / 2;

#[derive(Copy, Clone, Debug)]
struct RangeRecord {
    id: u16,

    tweak_current: u128,
    tweak_end: u128,
    key_progress: u128,
    key_committed_progress: u128
}

impl RangeRecord {

    fn from_value(k: u16, v: &(u128, u128, u128, u128)) -> Self {
        Self {
            id: k,
            tweak_current: v.0,
            tweak_end: v.1,
            key_progress: v.2,
            key_committed_progress: v.3,
        }
    }

    fn new(id: u16) -> Self {

        let tweak_start: u128 = id as u128 * TWEAKS_PER_RANGE;
        Self {
            id: id,
            tweak_current: tweak_start,
            tweak_end: tweak_start.strict_add(TWEAKS_PER_RANGE_ID_VICE) ,
            key_progress: 0,
            key_committed_progress: 0
        }
    }

    fn tweaks_left(&self) -> u128 {
        let t_left = (self.tweak_end - self.tweak_current) + 1;
        if t_left == 1 && self.key_committed_progress == u128::MAX {
            0u128
        }
        else {
            t_left
        }
    }

    fn completed_ratio(&self) -> f64 {
        let t_left = self.tweak_end - self.tweak_current;
        if t_left == 0 && self.key_committed_progress == u128::MAX {
            1.0f64
        }
        else {
            1.0f64 - ((t_left as f64 + 1.0)/ TWEAKS_PER_RANGE_F64)
        }
    }

    fn create_job(&mut self, job_id: u64, pref_job_size: u64) -> JobRecord {
        const MAX_U64_AS_U128: u128 = u64::MAX as u128;
        let start_key = self.key_progress + 1; // Key progrees point to the LAST key in previos job

        // using here self.key_progress enshures proper job len
        // in client we just need increment start_key this many times.
        let job_len = pref_job_size.min(MAX_U64_AS_U128.min(u128::MAX - self.key_progress) as u64);

        // update range progress
        // New key_progress point to end of current job
        self.key_progress += job_len as u128;

        if job_len == 0 {
            panic!("Job len should not ended as 0. We should never call create_job on busy range!");
        }

        JobRecord {
            id: job_id,
            range_id: self.id,
            tweak_key: self.tweak_current,
            start_key: start_key,
            len: job_len,
            start_time: get_timestump(),
        }
    }

    // return true if after commiting range is active for new jobs
    fn commit_work(&mut self, work_len: u64) -> bool {
        self.key_committed_progress = self.key_committed_progress.strict_add(work_len as u128);
        
        // if we finished one tweak bruteforcing.
        if self.key_committed_progress == u128::MAX {
            // Check if range is finished
            if self.tweak_current == self.tweak_end {
                false
            }
            else {
                // Increment key
                self.key_progress = 0;
                self.key_committed_progress = 0;
                self.tweak_current += 1;
                true
            }
        }
        else if self.key_progress == u128::MAX {
            // range is waiting for jobs to finish
            false
        }
        else {
            true
        }
    }

    fn value(&self) -> (u128, u128, u128, u128) {
        (self.tweak_current, self.tweak_end, self.key_progress, self.key_committed_progress)
    }
}

#[derive(Debug)]
struct RangesTable<'a> {
    ranges: redb::Table<'a, u16, (u128, u128, u128, u128)>,
    ranges_to_use: redb::Table<'a, u16, ()>,
}

impl<'a> RangesTable<'a> {
    fn new(
        ranges: redb::Table<'a, u16, (u128, u128, u128, u128)>,
        ranges_to_use: redb::Table<'a, u16, ()>,
    ) -> Self {
        RangesTable {
            ranges: ranges,
            ranges_to_use: ranges_to_use,
        }
    }

    fn open_rw(trx: &'a redb::WriteTransaction) -> Result<Self, redb::Error> {
        Ok(RangesTable {
            ranges: trx.open_table(RANGES)?,
            ranges_to_use: trx.open_table(RANGES_TO_USE)?,
        })
    }

    fn get_range(&self, k: u16) -> Result<Option<RangeRecord>, redb::Error> {
        Ok(self
            .ranges
            .get(k)?
            .and_then(|ag_v| Some(RangeRecord::from_value(k, &ag_v.value()))))
    }

    fn get_useful_ranges(&self) -> Result<Vec<RangeRecord>, redb::Error> {
        let mut u_ranges: Vec<RangeRecord> = Vec::with_capacity(self.ranges_to_use.len()? as usize);
        for kres in self.ranges_to_use.iter()? {
            u_ranges.push(
                self.get_range(kres?.0.value())?
                    .expect("Randge id from ranges_to_use should allways be present in ranges table"),
            );
        }

        Ok(u_ranges)
    }

    fn update_range(&mut self, range: &RangeRecord) -> Result<(), redb::Error> {
        self.ranges.insert(range.id, range.value())?;
        // if range key in progress reach max values - remove it from to_use table
        if range.key_progress == u128::MAX {
            self.ranges_to_use.remove(range.id)?;
        }
        Ok(())
    }

    fn commit_job(&mut self, job: &JobRecord) -> Result<(), redb::Error> {

        let mut range_rec = self.get_range(job.range_id)?
                    .expect("Randge id is not in ranges_table!");
        
        let is_active = range_rec.commit_work(job.len);

        // update range in db
        self.ranges.insert(range_rec.id, range_rec.value())?;

        if is_active {
            self.ranges_to_use.insert(range_rec.id, ())?;
        }
        else {
            self.ranges_to_use.remove(range_rec.id)?;
        }

        Ok(())
    }

    fn compute_progress(&self) -> Result<f64, redb::StorageError> {
        let mut mean_prog: f64 = 0.0;
        for range_res in self.ranges.iter()? {
            let ag = range_res?;
            let range = RangeRecord::from_value(ag.0.value(), &ag.1.value());
            mean_prog += range.completed_ratio();
        }

        Ok(mean_prog / 65536.0f64)
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
        let ranges_to_choose = ranges_table.get_useful_ranges()?;
        if ranges_to_choose.len() == 0 {
            println!("No ranges to choose from!! This can happed if work is sone");
            return Ok(None);
        }

        // randomly choosing range from avaylable ranges
        let mut range_to_use = ranges_to_choose[rand::rng().random_range(0..ranges_to_choose.len())];

        // if we found range - proceed. If None - means we already done.
        let new_job = range_to_use.create_job(job_id_to_use, pref_job_size);

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


// returns false if no such job found in DB
fn db_commit_job(db: Database, job_id: u64) -> Result<bool, redb::Error> {
    let transct: redb::WriteTransaction = db.begin_write()?;

    let job_found = {
        let mut jobs_table = transct.open_table(JOBS_TABLE)?;

        // 1. get job record
        let maybe_job_rec = jobs_table
            .get(job_id)?
            .and_then(|ag| Some(JobRecord::from_data(job_id, &ag.value())));

        if let Some(job_rec) = maybe_job_rec {
            // 2. Remove job from jobs_table and add job_id to jobs_free_ids table
            jobs_table.remove(job_id)?;

            // add job_id to jobs_free_ids table
            let mut jobs_free_ids = transct.open_table(JOBS_FREE_IDS)?;
            jobs_free_ids.insert(job_id, ())?;

            let mut ranges_table = RangesTable::open_rw(&transct)?;
            // commit job to range. Panik if overflow.
            ranges_table.commit_job(&job_rec)?;
            true
        }
        else {
            false
        }
    };
    transct.commit()?;

    return Ok(job_found); // Job already done by someone else
}


fn db_get_progress(db: Database) -> Result<f64, redb::Error> {
    let trx: redb::ReadTransaction = db.begin_read()?;
    let progress = {
        let ranges_t = trx.open_table(RANGES)?;
        let mut mean_prog: f64 = 0.0;
        for range_res in ranges_t.iter()? {
            let ag = range_res?;
            let range = RangeRecord::from_value(ag.0.value(), &ag.1.value());
            mean_prog += range.completed_ratio();
        }

        mean_prog / 65536.0f64
    };
    trx.close()?;
    return Ok(progress);
}


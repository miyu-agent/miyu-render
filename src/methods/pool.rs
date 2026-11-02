//! 画的那一套（[`Engine`]）借出去画、画完还回来（审计第四轮 R-2）。
//!
//! 原来画的线程拿着锁画到完：超时的那一张没人等了还占着锁，后来的请求排在它后面、等锁的时间也算进自己的限时，一句「你好」也
//! 跟着超时。现在锁只在借、还的那一下拿着：
//!
//! - 一次只替一个还在等的请求画（`live`），后来的排队，等的时间算进自己的限时（同原来）。
//! - 等的那一方超时了，那一张就算「没人要了」（`stray`）：它接着画完（线程停不下来），可不再挡路，排着的另开一套接着画。
//! - 没人要了还在画的最多 [`MAX_STRAY`] 张：都占满了、又没有闲着的那一套，当场回忙，不再另开（每一套各占一份字体库和一个核）。
//! - 字体换了（[`Pool::forget`]）扔掉闲着的那一套；借出去的是旧字体，画完不还。

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use crate::render::Engine;

/// 没人要了还在画的最多几张。
pub const MAX_STRAY: usize = 2;

/// 借不到的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// 排到限时还没轮到。
    Late,
    /// 没人要了还在画的占满了，没有闲着的那一套。
    Busy,
}

/// 借出去的一次：`engine` 空的是要新开一套。
pub struct Loan {
    pub id: u64,
    pub engine: Option<Engine>,
    generation: u64,
}

#[derive(Default)]
struct Slots {
    /// 闲着的那一套（画完还回来的）。
    idle: Option<Engine>,
    /// 正替一个还在等的请求画的那一次。
    live: Option<u64>,
    /// 没人要了、还在画的几张。
    stray: usize,
    /// 字体换了一次加一。
    generation: u64,
    /// 下一次借的编号。
    next: u64,
}

/// 画的那一套和借、还的账。
#[derive(Default)]
pub struct Pool {
    slots: Mutex<Slots>,
    changed: Condvar,
}

impl Pool {
    fn slots(&self) -> MutexGuard<'_, Slots> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 借一套来画：前面有人在画的排到 `deadline` 为止。
    ///
    /// # Errors
    ///
    /// 排到限时还没轮到（[`Refused::Late`]）；没人要了还在画的占满了（[`Refused::Busy`]）。
    pub fn borrow(&self, deadline: Instant) -> Result<Loan, Refused> {
        let mut slots = self.slots();
        loop {
            if slots.live.is_none() {
                if slots.idle.is_none() && slots.stray >= MAX_STRAY {
                    return Err(Refused::Busy);
                }
                let id = slots.next;
                slots.next += 1;
                slots.live = Some(id);
                return Ok(Loan {
                    id,
                    engine: slots.idle.take(),
                    generation: slots.generation,
                });
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(Refused::Late);
            }
            slots = self
                .changed
                .wait_timeout(slots, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// 画完了（画崩了的 `engine` 是空的）：字体没换、没有闲着的，留着下一次用。
    pub fn give_back(&self, loan: Loan) {
        let mut slots = self.slots();
        if slots.live == Some(loan.id) {
            slots.live = None;
        } else {
            slots.stray = slots.stray.saturating_sub(1);
        }
        if loan.generation == slots.generation && slots.idle.is_none() {
            slots.idle = loan.engine;
        }
        drop(slots);
        self.changed.notify_all();
    }

    /// 等的那一方超时了：编号 `id` 的那一张没人要了，不再挡后来的。已经画完还回来的不算。
    pub fn abandon(&self, id: u64) {
        let mut slots = self.slots();
        if slots.live == Some(id) {
            slots.live = None;
            slots.stray += 1;
        }
        drop(slots);
        self.changed.notify_all();
    }

    /// 字体换了：闲着的扔掉，借出去的画完不还。
    pub fn forget(&self) {
        let mut slots = self.slots();
        slots.idle = None;
        slots.generation += 1;
    }
}

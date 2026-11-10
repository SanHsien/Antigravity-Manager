//! 测试 determine_retry_strategy 和 should_rotate_account 的所有分支，
//! 重点覆盖 404 重试与账号轮换逻辑。

use crate::proxy::handlers::common::{
    determine_retry_strategy, should_rotate_account, RetryStrategy,
};
use std::time::Duration;

// ===== determine_retry_strategy =====

#[test]
fn test_retry_strategy_404() {
    // 404 is treated as "model not found" and never retried or rotated, whatever the body says
    // (same as upstream's test of the same name).
    for body in ["", "model not found", "account permission not synced"] {
        let strategy = determine_retry_strategy(404, body, false);
        assert!(
            matches!(strategy, RetryStrategy::NoRetry),
            "Expected NoRetry for 404 ({body:?}), got {:?}",
            strategy
        );
    }
}

#[test]
fn test_retry_strategy_429_no_delay() {
    let strategy = determine_retry_strategy(429, "rate limited", false);
    // Single-account default (pool_size = 1): wait 3s in place once instead of a linear backoff.
    assert!(
        matches!(strategy, RetryStrategy::GraceRetry(d) if d == Duration::from_millis(3000)),
        "Expected GraceRetry(3000ms), got {:?}",
        strategy
    );
}

#[test]
fn test_retry_strategy_503() {
    let strategy = determine_retry_strategy(503, "", false);
    // The fork keeps 5s..30s; upstream's shorter 1s..4s comes from PR #3634, which REVIEW defers.
    assert!(
        matches!(
            strategy,
            RetryStrategy::ExponentialBackoff {
                base_ms: 5000,
                max_ms: 30000
            }
        ),
        "Expected ExponentialBackoff {{ base_ms: 5000, max_ms: 30000 }}, got {:?}",
        strategy
    );
}

#[test]
fn test_retry_strategy_529() {
    let strategy = determine_retry_strategy(529, "", false);
    // The fork keeps 5s..30s; upstream's shorter 1s..4s comes from PR #3634, which REVIEW defers.
    assert!(
        matches!(
            strategy,
            RetryStrategy::ExponentialBackoff {
                base_ms: 5000,
                max_ms: 30000
            }
        ),
        "Expected ExponentialBackoff {{ base_ms: 5000, max_ms: 30000 }}, got {:?}",
        strategy
    );
}

#[test]
fn test_retry_strategy_500() {
    let strategy = determine_retry_strategy(500, "", false);
    assert!(
        matches!(strategy, RetryStrategy::LinearBackoff { base_ms: 3000 }),
        "Expected LinearBackoff {{ base_ms: 3000 }}, got {:?}",
        strategy
    );
}

#[test]
fn test_retry_strategy_401_403() {
    for status in [401, 403] {
        let strategy = determine_retry_strategy(status, "", false);
        match strategy {
            RetryStrategy::FixedDelay(d) => assert_eq!(d, Duration::from_millis(200)),
            other => panic!("Expected FixedDelay(200ms) for {}, got {:?}", status, other),
        }
    }
}

#[test]
fn test_retry_strategy_other() {
    for status in [200, 201, 301, 418, 502] {
        let strategy = determine_retry_strategy(status, "", false);
        assert!(
            matches!(strategy, RetryStrategy::NoRetry),
            "Expected NoRetry for {}, got {:?}",
            status,
            strategy
        );
    }
}

#[test]
fn test_retry_strategy_400_thinking_signature() {
    let signatures = [
        "Invalid `signature` for thinking",
        "Error with thinking.signature",
        "thinking.thinking block failed",
        "Corrupted thought signature detected",
    ];
    for sig in signatures {
        let strategy = determine_retry_strategy(400, sig, false);
        match strategy {
            RetryStrategy::FixedDelay(d) => assert_eq!(d, Duration::from_millis(200)),
            other => panic!(
                "Expected FixedDelay(200ms) for 400 + '{}', got {:?}",
                sig, other
            ),
        }
    }
}

#[test]
fn test_retry_strategy_400_no_signature() {
    let strategy = determine_retry_strategy(400, "bad request", false);
    assert!(
        matches!(strategy, RetryStrategy::NoRetry),
        "Expected NoRetry for 400 without signature, got {:?}",
        strategy
    );
}

// ===== should_rotate_account =====

#[test]
fn test_rotate_account_true_cases() {
    for status in [429, 401, 403, 404, 500, 503, 529] {
        assert!(
            should_rotate_account(status, None),
            "Expected should_rotate_account({}) == true",
            status
        );
    }
}

#[test]
fn test_rotate_account_false_cases() {
    for status in [400, 200, 502] {
        assert!(
            !should_rotate_account(status, None),
            "Expected should_rotate_account({}) == false",
            status
        );
    }
}

// ===== 自适应多轮次与单账号等待综合回归单测 (Refs #3485) =====

#[test]
fn test_calculate_max_retry_attempts_adaptive() {
    use crate::proxy::handlers::common::calculate_max_retry_attempts;
    assert_eq!(calculate_max_retry_attempts(0), 3);
    assert_eq!(calculate_max_retry_attempts(1), 3);
    assert_eq!(calculate_max_retry_attempts(2), 4);
    assert_eq!(calculate_max_retry_attempts(3), 6);
    assert_eq!(calculate_max_retry_attempts(5), 10);
    assert_eq!(calculate_max_retry_attempts(8), 12);
    assert_eq!(calculate_max_retry_attempts(20), 12);
}

#[test]
fn test_adaptive_retry_single_account_waits_quota_reset_delay() {
    use crate::proxy::handlers::common::determine_retry_strategy_adaptive;
    let err_json = r#"{"error":{"message":"Resource has been exhausted (e.g. check quota).","details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","quotaResetDelay":"4s"}]}}"#;

    // 单账号 (pool_size = 1): 绝不 50ms 闪电刷死，必须原地 GraceRetry 等待 quotaResetDelay (4s + 200ms = 4200ms)
    let strategy = determine_retry_strategy_adaptive(429, err_json, None, false, true, 0, 1);
    match strategy {
        RetryStrategy::GraceRetry(d) => assert_eq!(d, Duration::from_millis(4200)),
        other => panic!("Expected GraceRetry(4200ms), got {:?}", other),
    }
}

#[test]
fn test_adaptive_retry_multi_account_round_1_fast_rotates() {
    use crate::proxy::handlers::common::determine_retry_strategy_adaptive;
    let err_json = r#"{"error":{"message":"Resource has been exhausted","details":[{"quotaResetDelay":"4s"}]}}"#;

    // 多账号 Round 1 (attempt = 0, pool_size = 3): 闪电轮换 (50ms)，优先切向号池中其他健康账号
    let strategy = determine_retry_strategy_adaptive(429, err_json, None, false, true, 0, 3);
    match strategy {
        RetryStrategy::FixedDelay(d) => assert_eq!(d, Duration::from_millis(50)),
        other => panic!("Expected FixedDelay(50ms), got {:?}", other),
    }
}

#[test]
fn test_adaptive_retry_multi_account_round_2_small_gap_micro_waits() {
    use crate::proxy::handlers::common::determine_retry_strategy_adaptive;
    let err_json = r#"{"error":{"message":"Resource has been exhausted","details":[{"quotaResetDelay":"3s"}]}}"#;

    // 多账号 Round 2 (attempt = 3, pool_size = 3): 全池已试过一遍，遇到小间隙 (3s <= 5s)，小等并原地重试
    let strategy = determine_retry_strategy_adaptive(429, err_json, None, false, true, 3, 3);
    match strategy {
        RetryStrategy::GraceRetry(d) => assert_eq!(d, Duration::from_millis(3200)),
        other => panic!("Expected GraceRetry(3200ms), got {:?}", other),
    }
}

#[test]
fn test_adaptive_retry_multi_account_round_2_large_gap_rotates_if_more_than_two_accounts() {
    use crate::proxy::handlers::common::determine_retry_strategy_adaptive;
    let err_json = r#"{"error":{"message":"Resource has been exhausted","details":[{"quotaResetDelay":"15s"}]}}"#;

    // 多账号 Round 2 (attempt = 3, pool_size = 3): 延迟为 15s (> 5s) 且还有其他账号，继续闪电轮换寻找已恢复账号
    let strategy = determine_retry_strategy_adaptive(429, err_json, None, false, true, 3, 3);
    match strategy {
        RetryStrategy::FixedDelay(d) => assert_eq!(d, Duration::from_millis(50)),
        other => panic!("Expected FixedDelay(50ms), got {:?}", other),
    }
}

#[test]
fn test_adaptive_retry_explicit_hard_quota_rotates_full_pool_and_backs_off() {
    use crate::proxy::handlers::common::determine_retry_strategy_adaptive;
    // 明确的账号级硬配额耗尽返回（包含确切的配额耗尽关键字，且没有 quotaResetDelay）
    let hard_quota_429 = r#"{"error":{"code":429,"message":"You have exceeded your current quota. Please check your plan and billing details."}}"#;

    let pool_size = 3;
    // Round 1 (attempt 0, 1, 2): 账号级额度枯竭，全池快切寻找有额度账号 (50ms)
    for attempt in 0..pool_size {
        let s = determine_retry_strategy_adaptive(
            429,
            hard_quota_429,
            None,
            false,
            true,
            attempt,
            pool_size,
        );
        match s {
            RetryStrategy::FixedDelay(d) => assert_eq!(
                d,
                Duration::from_millis(50),
                "Attempt {} should fast rotate in round 1",
                attempt
            ),
            other => panic!(
                "Attempt {} expected FixedDelay(50ms), got {:?}",
                attempt, other
            ),
        }
    }

    // Round 2 (attempt 3, 4, 5): 全池账号均已耗尽，必须进入第二轮线性温和退避 (2s/4s/5s 封顶)，
    // 不能继续每次 50ms 空转刷死整个池
    for (attempt, expected_ms) in [(3usize, 2000u64), (4, 4000), (5, 5000)] {
        let s = determine_retry_strategy_adaptive(
            429,
            hard_quota_429,
            None,
            false,
            true,
            attempt,
            pool_size,
        );
        match s {
            RetryStrategy::FixedDelay(d) => assert_eq!(
                d,
                Duration::from_millis(expected_ms),
                "Round 2 attempt {} should enter gentle linear backoff",
                attempt
            ),
            other => panic!(
                "Round 2 attempt {} expected FixedDelay({}ms), got {:?}",
                attempt, expected_ms, other
            ),
        }
    }
}

#[test]
fn test_per_request_grace_cap_stops_same_account_retry_loop_in_round_2() {
    use crate::proxy::handlers::common::RequestRetryState;
    // 429 帶 3s 的 quotaResetDelay：第二輪的小間隙分支會回 GraceRetry（原地小等後重試同帳號）。
    let err_json = r#"{"error":{"message":"Resource has been exhausted","details":[{"quotaResetDelay":"3s"}]}}"#;
    let pool_size = 3;

    // 快取優先模式允許 Grace，但同一個帳號在同一個請求內只能 Grace 一次：
    // 第二輪 attempt 3 原地小等；attempt 4、5 必須改走輪換／退避，不能再對同一個冷卻中的帳號原地重試。
    let mut state = RequestRetryState::default();
    let mut graces = 0;
    let mut strategies = Vec::new();
    for attempt in pool_size..(pool_size * 2) {
        let s = state.determine_strategy_adaptive_gated(
            "acct-a", 429, err_json, None, false, attempt, pool_size, true,
        );
        if matches!(s, RetryStrategy::GraceRetry(_)) {
            graces += 1;
        }
        strategies.push(s);
    }
    assert_eq!(
        graces, 1,
        "same account must be grace-retried at most once per request, got {:?}",
        strategies
    );
    match &strategies[1] {
        RetryStrategy::FixedDelay(d) => assert_eq!(*d, Duration::from_millis(50)),
        other => panic!(
            "attempt 4 expected FixedDelay(50ms) rotation, got {:?}",
            other
        ),
    }

    // 排程模式不允許 Grace（Balance 多帳號／PerformanceFirst）時，一次都不能 Grace。
    let mut gated_off = RequestRetryState::default();
    for attempt in pool_size..(pool_size * 2) {
        let s = gated_off.determine_strategy_adaptive_gated(
            "acct-a", 429, err_json, None, false, attempt, pool_size, false,
        );
        assert!(
            !matches!(s, RetryStrategy::GraceRetry(_)),
            "scheduling mode forbids grace, attempt {} got {:?}",
            attempt,
            s
        );
    }

    // 不同帳號各自有一次 Grace 額度。
    let mut per_account = RequestRetryState::default();
    for id in ["acct-a", "acct-b"] {
        let s = per_account.determine_strategy_adaptive_gated(
            id, 429, err_json, None, false, pool_size, pool_size, true,
        );
        assert!(
            matches!(s, RetryStrategy::GraceRetry(_)),
            "{} first grace",
            id
        );
    }
}

#[test]
fn test_per_request_grace_cap_single_account_degrades_to_fixed_delay() {
    use crate::proxy::handlers::common::RequestRetryState;
    // 單帳號池 (pool_size = 1) 的 429：同一帳號在同一個請求內只能 Grace 一次，
    // 第二次起改回固定退避，否則呼叫端會對同一個帳號無限原地重試。
    let with_delay = r#"{"error":{"details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"1s"}]}}"#;
    let without_delay = "rate limited";

    for body in [with_delay, without_delay] {
        let mut state = RequestRetryState::default();
        let first =
            state.determine_strategy_adaptive_gated("acct-a", 429, body, None, false, 0, 1, true);
        assert!(
            matches!(first, RetryStrategy::GraceRetry(_)),
            "first 429 on a single account is grace-retried, got {:?}",
            first
        );
        for _ in 0..3 {
            let again = state
                .determine_strategy_adaptive_gated("acct-a", 429, body, None, false, 0, 1, true);
            assert!(
                matches!(again, RetryStrategy::FixedDelay(_)),
                "grace budget spent: expected FixedDelay, got {:?} (body {:?})",
                again,
                body
            );
        }
        // 排程模式不允許 Grace 時，第一次也不能 Grace。
        let mut gated_off = RequestRetryState::default();
        let s = gated_off
            .determine_strategy_adaptive_gated("acct-a", 429, body, None, false, 0, 1, false);
        assert!(
            !matches!(s, RetryStrategy::GraceRetry(_)),
            "scheduling mode forbids grace, got {:?}",
            s
        );
    }
}

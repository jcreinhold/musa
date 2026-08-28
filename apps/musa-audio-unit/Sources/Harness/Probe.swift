/// The allocation probe, if `DYLD_INSERT_LIBRARIES` put it in this process.
///
/// Prompt 215 built the interposing library this reaches — `AllocProbe.c` in
/// the trial, which this project compiles rather than copies, because one
/// interposer that both harnesses agree on is what makes their numbers
/// comparable. `06-daw-boundary.md` §7 is what it exists to check: a render
/// block does not allocate, and Apple publishes no way to assert that.
///
/// It sees this process only. An out-of-process extension runs somewhere the
/// system spawned and this library was never inserted into, so every
/// allocation finding here is about the in-process load and says so.

import Foundation

enum AllocationProbe {
    private typealias Arm = @convention(c) (Int32) -> Void
    private typealias Count = @convention(c) () -> UInt64
    private typealias Reset = @convention(c) () -> Void

    private static let arm = unsafeBitCast(
        dlsym(UnsafeMutableRawPointer(bitPattern: -2), "musa_probe_arm"), to: Arm?.self
    )
    private static let count = unsafeBitCast(
        dlsym(UnsafeMutableRawPointer(bitPattern: -2), "musa_probe_allocations"), to: Count?.self
    )
    private static let reset = unsafeBitCast(
        dlsym(UnsafeMutableRawPointer(bitPattern: -2), "musa_probe_reset"), to: Reset?.self
    )

    /// Whether the probe is present *and* counting. A probe that loaded but
    /// does not observe a deliberate allocation is not evidence of anything.
    static var isWorking: Bool {
        guard let arm, let count, let reset else { return false }
        reset()
        arm(1)
        let sample = malloc(64)
        free(sample)
        arm(0)
        return count() > 0
    }

    /// Count the allocations that happen inside `body`.
    static func measure(_ body: () -> Void) -> UInt64? {
        guard let arm, let count, let reset else { return nil }
        reset()
        arm(1)
        body()
        arm(0)
        return count()
    }
}

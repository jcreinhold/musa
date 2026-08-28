/*  A caller written in Objective-C, holding the block it calls.
 *
 *  Measuring a render block from Swift measures the Swift caller too. Passing
 *  a Swift closure value into an Objective-C block parameter copies the block
 *  once per call, and driving two thousand blocks that way reads as an
 *  allocation that has nothing to do with the component. Prompt 215 measured
 *  exactly that, so the block is handed over once, at construction, and the
 *  loop that calls it lives here.
 */

#ifndef MUSA_AU_DRIVE_H
#define MUSA_AU_DRIVE_H

#import <AudioToolbox/AudioToolbox.h>
#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

/// Calls one component's render block, repeatedly, from compiled code.
@interface MusaAuDriver : NSObject

/// Take the block once. Every allocation this costs happens here.
- (instancetype)initWithRenderBlock:(AUInternalRenderBlock)renderBlock NS_DESIGNATED_INITIALIZER;

/// An empty block that returns `noErr` and touches nothing.
///
/// The baseline every allocation number is published beside: what the
/// harness's own call path costs, so that a number is about the component
/// rather than about the rig.
+ (instancetype)emptyDriver;

- (instancetype)init NS_UNAVAILABLE;

/// Call the block `rounds` times, advancing the sample time by `frames` each
/// time, and return the last status. Allocates nothing.
- (OSStatus)runWithFlags:(AudioUnitRenderActionFlags *)flags
               timestamp:(AudioTimeStamp *)timestamp
                  frames:(AUAudioFrameCount)frames
         outputBusNumber:(NSInteger)outputBusNumber
              outputData:(AudioBufferList *)outputData
                  events:(nullable const AURenderEvent *)events
                  rounds:(uint32_t)rounds;

@end

/// A MIDI destination written in Objective-C, for the same reason the driver
/// is: a component that emits into a Swift closure is measured together with
/// the bridging thunk that closure is called through, and that thunk copies
/// per call. A host block written here allocates nothing, so an allocation
/// the probe sees while the component renders is the component's.
///
/// It keeps a bounded record. A sink that grew an array per message would be
/// the thing the probe saw; past `capacity` it counts and stops keeping.
@interface MusaAuMidiSink : NSObject

/// Room for `capacity` messages of three bytes.
- (instancetype)initWithCapacity:(NSUInteger)capacity NS_DESIGNATED_INITIALIZER;
- (instancetype)init NS_UNAVAILABLE;

/// The block to install as the component's `midiOutputEventBlock`. Handed
/// over once; every allocation it costs happens here.
@property(nonatomic, readonly) AUMIDIOutputEventBlock block;

/// How many messages arrived, kept or not.
@property(nonatomic, readonly) NSUInteger count;

/// How many were kept.
@property(nonatomic, readonly) NSUInteger keptCount;

/// The sample offset of a kept message.
- (AUEventSampleTime)offsetAt:(NSUInteger)index NS_SWIFT_NAME(offset(at:));

/// One byte of a kept message. `byte` is 0, 1, or 2.
- (uint8_t)byteAt:(NSUInteger)index of:(NSUInteger)byte NS_SWIFT_NAME(byte(at:of:));

/// How many bytes a kept message carried.
- (NSUInteger)lengthAt:(NSUInteger)index NS_SWIFT_NAME(length(at:));

/// Forget everything, keeping the room.
- (void)reset;

@end

NS_ASSUME_NONNULL_END

#endif /* MUSA_AU_DRIVE_H */

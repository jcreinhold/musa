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

NS_ASSUME_NONNULL_END

#endif /* MUSA_AU_DRIVE_H */

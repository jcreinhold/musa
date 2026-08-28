#import "MusaTrialDrive.h"

@implementation MusaTrialDriver {
    AUInternalRenderBlock _renderBlock;
}

- (instancetype)initWithRenderBlock:(AUInternalRenderBlock)renderBlock {
    self = [super init];
    if (self) {
        _renderBlock = [renderBlock copy];
    }
    return self;
}

- (OSStatus)runWithFlags:(AudioUnitRenderActionFlags *)flags
               timestamp:(AudioTimeStamp *)timestamp
                  frames:(AUAudioFrameCount)frames
         outputBusNumber:(NSInteger)outputBusNumber
              outputData:(AudioBufferList *)outputData
                  events:(const AURenderEvent *)events
                  rounds:(uint32_t)rounds {
    AUInternalRenderBlock block = _renderBlock;
    OSStatus status = noErr;
    for (uint32_t round = 0; round < rounds; ++round) {
        timestamp->mSampleTime = (Float64)((uint64_t)round * frames);
        timestamp->mFlags = kAudioTimeStampSampleTimeValid;
        status = block(flags, timestamp, frames, outputBusNumber, outputData, events, NULL);
    }
    return status;
}

@end

#import "MusaAuDrive.h"

@implementation MusaAuDriver {
    AUInternalRenderBlock _renderBlock;
}

- (instancetype)initWithRenderBlock:(AUInternalRenderBlock)renderBlock {
    self = [super init];
    if (self) {
        _renderBlock = [renderBlock copy];
    }
    return self;
}

+ (instancetype)emptyDriver {
    return [[self alloc] initWithRenderBlock:^AUAudioUnitStatus(AudioUnitRenderActionFlags *actionFlags,
                                                        const AudioTimeStamp *timestamp,
                                                        AUAudioFrameCount frameCount, NSInteger outputBusNumber,
                                                        AudioBufferList *outputData,
                                                        const AURenderEvent *realtimeEventListHead,
                                                        AURenderPullInputBlock __unsafe_unretained pullInputBlock) {
        (void)actionFlags;
        (void)timestamp;
        (void)frameCount;
        (void)outputBusNumber;
        (void)outputData;
        (void)realtimeEventListHead;
        (void)pullInputBlock;
        return noErr;
    }];
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

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

@implementation MusaAuMidiSink {
    NSUInteger _capacity;
    AUEventSampleTime *_offsets;
    uint8_t *_bytes;
    NSUInteger *_lengths;
}

- (instancetype)initWithCapacity:(NSUInteger)capacity {
    self = [super init];
    if (self) {
        _capacity = capacity;
        _offsets = calloc(capacity, sizeof(AUEventSampleTime));
        _bytes = calloc(capacity * 3, sizeof(uint8_t));
        _lengths = calloc(capacity, sizeof(NSUInteger));
        __unsafe_unretained MusaAuMidiSink *sink = self;
        _block = [^AUAudioUnitStatus(AUEventSampleTime offset, uint8_t cable, NSInteger length,
                                     const uint8_t *bytes) {
            (void)cable;
            [sink take:offset length:length bytes:bytes];
            return noErr;
        } copy];
    }
    return self;
}

- (void)dealloc {
    free(_offsets);
    free(_bytes);
    free(_lengths);
}

/* Called from the render thread. Nothing here allocates. */
- (void)take:(AUEventSampleTime)offset length:(NSInteger)length bytes:(const uint8_t *)bytes {
    NSUInteger index = _count;
    _count += 1;
    if (index >= _capacity) {
        return;
    }
    _offsets[index] = offset;
    NSUInteger kept = length < 3 ? (NSUInteger)(length < 0 ? 0 : length) : 3;
    _lengths[index] = (NSUInteger)length;
    for (NSUInteger byte = 0; byte < kept; ++byte) {
        _bytes[index * 3 + byte] = bytes[byte];
    }
    _keptCount = index + 1;
}

- (AUEventSampleTime)offsetAt:(NSUInteger)index {
    return index < _keptCount ? _offsets[index] : 0;
}

- (uint8_t)byteAt:(NSUInteger)index of:(NSUInteger)byte {
    return (index < _keptCount && byte < 3) ? _bytes[index * 3 + byte] : 0;
}

- (NSUInteger)lengthAt:(NSUInteger)index {
    return index < _keptCount ? _lengths[index] : 0;
}

- (void)reset {
    _count = 0;
    _keptCount = 0;
}

@end

//  The framework's umbrella header.
//
//  Both extensions and the harness link this one framework, so the Music
//  Device, the MIDI Processor, and the code that measures them are the same
//  code — which is what makes an in-process and an out-of-process reading
//  comparable at all.

#import <Foundation/Foundation.h>

FOUNDATION_EXPORT double MusaTrialKitVersionNumber;
FOUNDATION_EXPORT const unsigned char MusaTrialKitVersionString[];

#import <MusaTrialKit/MusaTrialABI.h>
#import <MusaTrialKit/MusaTrialDrive.h>

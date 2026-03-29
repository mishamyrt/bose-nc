#import <Foundation/Foundation.h>
#import <IOBluetooth/IOBluetooth.h>

static const NSTimeInterval kChannelOpenTimeout    = 2.0;
static const NSTimeInterval kInterWriteDelay       = 0.05;
static const NSTimeInterval kResponseWaitTime      = 0.1;
static const NSTimeInterval kChannelCloseTimeout   = 2.0;
static const NSTimeInterval kSDPQueryTimeout       = 2.0;
static const NSTimeInterval kRunLoopGranularity    = 0.05;

@interface BoseRFCOMMDelegate : NSObject <IOBluetoothRFCOMMChannelDelegate> {
    @public
    BOOL _channelOpen;
    BOOL _channelClosed;
}
@property (nonatomic, strong) NSMutableData *receivedData;
@end

@implementation BoseRFCOMMDelegate

- (instancetype)init {
    self = [super init];
    if (self) {
        _receivedData = [NSMutableData data];
        _channelOpen = NO;
        _channelClosed = NO;
    }
    return self;
}

- (void)rfcommChannelOpenComplete:(IOBluetoothRFCOMMChannel *)rfcommChannel
                           status:(IOReturn)error {
    _channelOpen = (error == kIOReturnSuccess);
}

- (void)rfcommChannelData:(IOBluetoothRFCOMMChannel *)rfcommChannel
                     data:(void *)dataPointer
                   length:(size_t)dataLength {
    [_receivedData appendBytes:dataPointer length:dataLength];
}

- (void)rfcommChannelClosed:(IOBluetoothRFCOMMChannel *)rfcommChannel {
    _channelClosed = YES;
}

@end

static void run_loop_for(NSTimeInterval seconds) {
    [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:seconds]];
}

static int find_spp_channel(IOBluetoothDevice *device) {
    NSArray *services = [device services];
    if (!services) {
        [device performSDPQuery:nil];
        run_loop_for(kSDPQueryTimeout);
        services = [device services];
    }
    if (!services) return -1;

    IOBluetoothSDPUUID *sppUUID = [IOBluetoothSDPUUID uuid16:0x1101];

    for (IOBluetoothSDPServiceRecord *record in services) {
        if (![record matchesUUIDArray:@[sppUUID]]) continue;

        BluetoothRFCOMMChannelID channelID = 0;
        if ([record getRFCOMMChannelID:&channelID] == kIOReturnSuccess) {
            return (int)channelID;
        }
    }

    NSLog(@"bose-nc: no SPP (0x1101) service found. Available services:");
    for (IOBluetoothSDPServiceRecord *record in services) {
        NSLog(@"bose-nc:   - %@", [record getServiceName] ?: @"(unnamed)");
    }
    return -1;
}

static BOOL wait_for(BOOL *flag, NSTimeInterval timeout) {
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:timeout];
    while (!*flag && [[NSDate date] compare:deadline] == NSOrderedAscending) {
        [[NSRunLoop currentRunLoop]
            runUntilDate:[NSDate dateWithTimeIntervalSinceNow:kRunLoopGranularity]];
    }
    return *flag;
}

static void close_rfcomm(IOBluetoothRFCOMMChannel *rfcomm,
                         BoseRFCOMMDelegate *delegate) {
    [rfcomm closeChannel];
    wait_for(&delegate->_channelClosed, kChannelCloseTimeout);
    [rfcomm setDelegate:nil];
}

int bose_rfcomm_send(const char *bt_address,
                     const uint8_t *send_buf, int send_len,
                     uint8_t *out_buf, int out_capacity,
                     int rfcomm_channel, int send_count) {
    @autoreleasepool {
        NSString *addr = [[NSString stringWithUTF8String:bt_address]
                          stringByReplacingOccurrencesOfString:@":" withString:@"-"];
        IOBluetoothDevice *device = [IOBluetoothDevice deviceWithAddressString:addr];
        if (!device) {
            NSLog(@"bose-nc: device not found for address %@", addr);
            return -1;
        }

        int channel = rfcomm_channel;
        if (channel < 0) {
            channel = find_spp_channel(device);
            if (channel < 0) {
                NSLog(@"bose-nc: SPP service not found on %@", addr);
                return -2;
            }
        }

        BoseRFCOMMDelegate *delegate = [[BoseRFCOMMDelegate alloc] init];
        IOBluetoothRFCOMMChannel *rfcomm = nil;
        IOReturn result = [device openRFCOMMChannelSync:&rfcomm
                                          withChannelID:(BluetoothRFCOMMChannelID)channel
                                               delegate:delegate];

        if (result != kIOReturnSuccess || !rfcomm) {
            NSLog(@"bose-nc: failed to open RFCOMM channel %d (IOReturn %#x)", channel, result);
            return -3;
        }

        if (!wait_for(&delegate->_channelOpen, kChannelOpenTimeout)) {
            NSLog(@"bose-nc: timed out waiting for RFCOMM channel open callback");
            close_rfcomm(rfcomm, delegate);
            return -3;
        }

        if (send_count < 1) send_count = 1;
        for (int i = 0; i < send_count; i++) {
            result = [rfcomm writeSync:(void *)send_buf length:send_len];
            if (result != kIOReturnSuccess) {
                NSLog(@"bose-nc: RFCOMM write failed (IOReturn %#x)", result);
                close_rfcomm(rfcomm, delegate);
                return -4;
            }
            if (i < send_count - 1) {
                run_loop_for(kInterWriteDelay);
            }
        }

        run_loop_for(kResponseWaitTime);

        int received = (int)[delegate.receivedData length];
        if (received > 0 && out_buf && out_capacity > 0) {
            int copy_len = received < out_capacity ? received : out_capacity;
            memcpy(out_buf, [delegate.receivedData bytes], copy_len);
            received = copy_len;
        }

        close_rfcomm(rfcomm, delegate);

        return received;
    }
}

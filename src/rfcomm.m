#import <Foundation/Foundation.h>
#import <IOBluetooth/IOBluetooth.h>

// Delegate that receives RFCOMM data
@interface BoseRFCOMMDelegate : NSObject <IOBluetoothRFCOMMChannelDelegate>
@property (nonatomic) BOOL channelOpen;
@property (nonatomic, strong) NSMutableData *receivedData;
@property (nonatomic) BOOL channelClosed;
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

// Run the NSRunLoop for a given duration to process async callbacks
static void run_loop_for(NSTimeInterval seconds) {
    [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:seconds]];
}

// Find RFCOMM channel for the SPP service on a Bose device
static int find_spp_channel(IOBluetoothDevice *device) {
    NSArray *services = [device services];
    if (!services) {
        [device performSDPQuery:nil];
        run_loop_for(0.1);
        services = [device services];
    }
    if (!services) return -1;

    for (IOBluetoothSDPServiceRecord *record in services) {
        NSString *name = [record getServiceName];
        if (name && [name isEqualToString:@"SPP Dev"]) {
            BluetoothRFCOMMChannelID channelID = 0;
            IOReturn result = [record getRFCOMMChannelID:&channelID];
            if (result == kIOReturnSuccess) {
                return (int)channelID;
            }
            // Parse from protocol descriptor if getRFCOMMChannelID fails
            IOBluetoothSDPDataElement *proto = [record getAttributeDataElement:0x0004];
            if (proto) {
                NSArray *protoList = [proto getArrayValue];
                for (IOBluetoothSDPDataElement *entry in protoList) {
                    NSArray *elems = [entry getArrayValue];
                    if (elems.count >= 2) {
                        IOBluetoothSDPDataElement *uuidElem = elems[0];
                        IOBluetoothSDPUUID *uuid = [uuidElem getUUIDValue];
                        // RFCOMM UUID = 0x0003. IOBluetoothSDPUUID is an NSData subclass.
                        uint8_t rfcommUUID[] = {0x00, 0x03};
                        if (uuid && [(NSData *)uuid length] >= 2 &&
                            memcmp([(NSData *)uuid bytes], rfcommUUID, 2) == 0) {
                            IOBluetoothSDPDataElement *chElem = elems[1];
                            return (int)[[chElem getNumberValue] intValue];
                        }
                    }
                }
            }
        }
    }
    return -1;
}

/// Send a BMAP packet over RFCOMM and optionally receive a response.
/// Returns number of bytes written to out_buf, or -1 on error.
int bose_rfcomm_send(const char *bt_address,
                     const uint8_t *send_buf, int send_len,
                     uint8_t *out_buf, int out_capacity,
                     int rfcomm_channel) {
    @autoreleasepool {
        NSString *addr = [[NSString stringWithUTF8String:bt_address]
                          stringByReplacingOccurrencesOfString:@":" withString:@"-"];
        IOBluetoothDevice *device = [IOBluetoothDevice deviceWithAddressString:addr];
        if (!device) return -1;

        int channel = rfcomm_channel;
        if (channel < 0) {
            channel = find_spp_channel(device);
            if (channel < 0) return -2;
        }

        BoseRFCOMMDelegate *delegate = [[BoseRFCOMMDelegate alloc] init];
        IOBluetoothRFCOMMChannel *rfcomm = nil;
        IOReturn result = [device openRFCOMMChannelSync:&rfcomm
                                          withChannelID:(BluetoothRFCOMMChannelID)channel
                                               delegate:delegate];

        if (!rfcomm) return -3;

        run_loop_for(0.1);

        result = [rfcomm writeSync:(void *)send_buf length:send_len];
        if (result != kIOReturnSuccess) {
            [rfcomm closeChannel];
            return -4;
        }

        // Wait for response
        run_loop_for(0.1);

        int received = (int)[delegate.receivedData length];
        if (received > 0 && out_buf && out_capacity > 0) {
            int copy_len = received < out_capacity ? received : out_capacity;
            memcpy(out_buf, [delegate.receivedData bytes], copy_len);
            received = copy_len;
        }

        [rfcomm closeChannel];
        run_loop_for(0.1);

        return received;
    }
}

/// List connected Bose devices. Writes addresses into out_buf as null-terminated
/// strings separated by newlines. Returns number of devices found.
int bose_list_devices(char *out_buf, int out_capacity) {
    @autoreleasepool {
        NSArray *paired = [IOBluetoothDevice pairedDevices];
        int count = 0;
        int offset = 0;

        for (IOBluetoothDevice *device in paired) {
            if (![device isConnected]) continue;
            NSString *name = [device name];
            if (!name) continue;
            NSString *lower = [name lowercaseString];
            if (![lower containsString:@"bose"]) continue;

            NSString *addr = [[device addressString]
                              stringByReplacingOccurrencesOfString:@"-" withString:@":"];
            NSString *line = [NSString stringWithFormat:@"%@\t%@\n",
                              [addr uppercaseString], name];
            const char *cstr = [line UTF8String];
            int len = (int)strlen(cstr);

            if (offset + len < out_capacity) {
                memcpy(out_buf + offset, cstr, len);
                offset += len;
                count++;
            }
        }

        if (offset < out_capacity)
            out_buf[offset] = '\0';

        return count;
    }
}

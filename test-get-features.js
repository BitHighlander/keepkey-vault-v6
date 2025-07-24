const { invoke } = require('@tauri-apps/api/core');

async function testGetFeatures() {
    console.log('🧪 Testing get_features performance...\n');
    
    try {
        // First, get connected devices
        const devices = await invoke('get_connected_devices');
        console.log(`📱 Found ${devices.length} devices`);
        
        if (devices.length === 0) {
            console.log('❌ No devices connected');
            return;
        }
        
        const deviceId = devices[0].device_id;
        console.log(`🎯 Testing with device: ${deviceId}\n`);
        
        // Test 10 sequential get_features calls
        const times = [];
        
        for (let i = 0; i < 10; i++) {
            const start = Date.now();
            
            try {
                const features = await invoke('get_features', { deviceId });
                const elapsed = Date.now() - start;
                times.push(elapsed);
                
                console.log(`✅ Call ${i + 1}: ${elapsed}ms`);
                
                if (i === 0) {
                    console.log('   First call (creates transport)');
                    console.log(`   Device: ${features.label || 'Unnamed'}`);
                    console.log(`   Version: ${features.major_version}.${features.minor_version}.${features.patch_version}`);
                }
            } catch (e) {
                console.error(`❌ Call ${i + 1} failed:`, e);
            }
        }
        
        // Calculate statistics
        const firstCall = times[0];
        const subsequentCalls = times.slice(1);
        const avgSubsequent = subsequentCalls.reduce((a, b) => a + b, 0) / subsequentCalls.length;
        
        console.log('\n📊 Performance Summary:');
        console.log(`   First call (transport creation): ${firstCall}ms`);
        console.log(`   Average subsequent calls: ${avgSubsequent.toFixed(1)}ms`);
        console.log(`   Speed improvement: ${(firstCall / avgSubsequent).toFixed(1)}x faster`);
        
        // Expected results with persistent transport:
        // - First call: 200-400ms (transport creation)
        // - Subsequent calls: 50-100ms (reusing transport)
        // - Should see 3-5x improvement
        
    } catch (error) {
        console.error('❌ Test failed:', error);
    }
}

// Run the test
testGetFeatures(); 
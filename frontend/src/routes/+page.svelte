<script lang="ts">
    import { onMount } from 'svelte';
    import LatencyChart from '$lib/components/LatencyChart.svelte';
    import MainCharts from '$lib/components/MainCharts.svelte';

    type Record = {
        interval: number;
        end_of_interval: number;
        latency: number;
        rank: number;
        has_trip: boolean;
    };

    type Percentiles = {
        p50: number;
        p90: number;
        p99: number;
    };

    type SystemStats = {
        timestamp: number;
        interval_stats: Percentiles;
        latency_stats: Percentiles;
        sample_count: number;
    };

    type ScoredBus = {
        bus_id: string;
        score: number;
        history: Record[];
    };

    let stats: SystemStats[] = $state([]);
    let anomalies: ScoredBus[] = $state([]);
    let error: string | null = $state(null);

    // Time Range Selector
    let selectedTimeRange: number | null = $state(30 * 60); // Default 30m

    async function fetchData() {
        try {
            // Determine params
            let params = new URLSearchParams();
            if (selectedTimeRange !== null) {
                const since = Math.floor(Date.now() / 1000 - selectedTimeRange);
                params.set('since', String(since));
                
                let step = 1;
                if (selectedTimeRange > 4 * 3600) {
                    step = 30; // >4h -> 5m
                } else if (selectedTimeRange === 4 * 3600) {
                    step = 6;  // 4h -> 1m
                } else if (selectedTimeRange === 3600) {
                    step = 2;  // 1h -> 20s
                }
                params.set('step', String(step));
            } else {
                // Max
                params.set('step', '30');
            }

            const [statsRes, anomaliesRes] = await Promise.all([
                fetch(`https://lametrobuspingapi.catenarymaps.org/stats?${params.toString()}`, {
                    mode: 'cors'
                }),
                fetch(`https://lametrobuspingapi.catenarymaps.org/anomalies?min_rank=90&${params.toString()}`, {
                    mode: 'cors'
                })
            ]);

            if (statsRes.ok) stats = await statsRes.json();
            if (anomaliesRes.ok) anomalies = await anomaliesRes.json();
        } catch (e) {
            error = String(e);
        }
    }

    $effect(() => {
        // Refetch when time range changes
        // Using $derived or explicit reaction
        // Simple way: rely on selectedTimeRange being tracked.
        // But fetchData is async and not reactive by default unless called in effect.
        // We can just call fetchData whenever selectedTimeRange changes.
    });
    
    // Watch selectedTimeRange
    $effect(() => {
        // This will run when selectedTimeRange changes
        // But we need to make sure we don't double fetch on mount if mount calls it too.
        // On mount, we set interval.
        // Let's just call fetchData().
        // Note: We need to use the value of selectedTimeRange inside.
        // By reading it here, we subscribe to it.
        const _ = selectedTimeRange;
        fetchData();
    });

    onMount(() => {
        // fetchData is called by the effect on mount?
        // standard pattern:
        // fetchData(); // Effect handles initial call
        const interval = setInterval(fetchData, 10000);
        return () => clearInterval(interval);
    });



    let minTime: number | null = $state(null);
    let maxTime: number | null = $state(null);
    let hoveredTime: number | null = $state(null);

    function handleZoom(min: number, max: number) {
        minTime = min;
        maxTime = max;
    }

    function resetZoom() {
        minTime = null;
        maxTime = null;
    }

    function handleHover(time: number | null) {
        hoveredTime = time;
    }

    $effect(() => {
        const snapParams = $state.snapshot(stats);
        if (snapParams.length > 0) {
            const first = snapParams[0];
            console.log('Stats[0] keys:', Object.keys(first));
            if (first.latency_stats) {
                console.log('Stats[0].latency_stats keys:', Object.keys(first.latency_stats));
                console.log('Stats[0].latency_stats values:', first.latency_stats);
            }
            if (first.interval_stats) {
                console.log('Stats[0].interval_stats keys:', Object.keys(first.interval_stats));
            }
        }
    });

</script>

<svelte:head>
    <link rel="icon" href="https://lametrobusping.catenarymaps.org/logo.svg">
    <title>LA Metro Bus Ping</title>
    <meta property="og:title" content="LA Metro Bus Ping" />
    <meta property="og:type" content="website" />
    <meta property="og:image" content="https://lametrobusping.catenarymaps.org/thumbnailbusping.png" />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:image" content="https://lametrobusping.catenarymaps.org/thumbnailbusping.png" />
</svelte:head>

<div class="bg-gray-50 min-h-screen font-sans">
    <!-- Sticky Header -->
    <header class="sticky top-0 left-0 right-0 z-50 bg-white border-b border-gray-200 shadow-sm px-8 py-4 flex items-center justify-between">
        <div class="flex flex-row items-center space-x-2">
            <img src="https://catenarymaps.org/logomark.svg" alt="Catenary Maps" class="h-6">
            <div class="">|</div>
            <img src="/Lametro.svg" class="h-7"/>
            <h1 class="text-xl font-bold">
                Bus Ping
            </h1>
        </div>
        
        <div class="flex items-center space-x-4">
             <div class="flex bg-gray-100 rounded p-1 space-x-1">
                {#each [
                    { label: '30m', val: 30 * 60 }, 
                    { label: '1h', val: 60 * 60 }, 
                    { label: '4h', val: 4 * 60 * 60 },
                    { label: '12h', val: 12 * 60 * 60 },
                    { label: '24h', val: 24 * 60 * 60 },
                    { label: 'Max', val: null }
                ] as range}
                    <button 
                        class="px-2 py-1 rounded text-xs font-medium transition-colors {selectedTimeRange === range.val ? 'bg-white shadow text-blue-600' : 'text-gray-500 hover:text-gray-700'}"
                        onclick={() => selectedTimeRange = range.val}
                    >
                        {range.label}
                    </button>
                {/each}
            </div>

            <button 
                class="bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={resetZoom}
                disabled={minTime === null}
            >
                Reset Zoom
            </button>
            <span class="text-sm text-gray-400">v0.1.0</span>
        </div>
    </header>

    <main class="p-8">
        {#if error}
            <div class="bg-red-100 p-4 mb-4 rounded text-red-700">{error}</div>
        {/if}

        <MainCharts 
            stats={stats}
            minTime={minTime}
            maxTime={maxTime}
            hoveredTime={hoveredTime}
            onZoom={handleZoom}
            onHover={handleHover}
        />

        <div>
            <h2 class="text-xl font-semibold mb-4">Top Anomalous Buses (>P80)</h2>
            <div class="space-y-4">
                {#each anomalies as bus (bus.bus_id)}
                    <div class="bg-white p-4 rounded shadow grid grid-cols-1 md:grid-cols-[150px_1fr] gap-4 items-center overflow-hidden">
                        <div>
                            <div class="text-lg font-bold text-gray-900">{bus.bus_id}</div>
                            <div class="text-sm text-gray-500">Score: <span class="font-mono">{bus.score}</span></div>
                            <div class="text-xs text-gray-400 mt-1">{bus.history.length} data points</div>
                        </div>
                        
                        <div class="h-40 w-full min-w-0"> <!-- min-w-0 is key for grid child scaling -->
                            <LatencyChart 
                                datasets={[
                                    { 
                                        label: 'Latency', 
                                        data: bus.history.map(r => ({ x: r.end_of_interval, y: r.latency, has_trip: r.has_trip })), 
                                        color: '#ef4444' 
                                    },
                                    { 
                                        label: 'Interval', 
                                        data: bus.history.map(r => ({ x: r.end_of_interval, y: r.interval, has_trip: r.has_trip })), 
                                        color: '#3b82f6' 
                                    }
                                ]}
                                height="h-full"
                                showLegend={true}
                                min={minTime} 
                                max={maxTime} 
                                onZoom={handleZoom}
                                hoveredTime={hoveredTime}
                                onHover={handleHover}
                            />
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    </main>
</div>

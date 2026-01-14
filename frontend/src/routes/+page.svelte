<script lang="ts">
    import { onMount } from 'svelte';
    import LatencyChart from '$lib/components/LatencyChart.svelte';
    import MainCharts from '$lib/components/MainCharts.svelte';

    type Record = {
        interval: number;
        end_of_interval: number;
        latency: number;
        rank: number;
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

    async function fetchData() {
        try {
            const [statsRes, anomaliesRes] = await Promise.all([
                fetch('https://lametrobuspingapi.catenarymaps.org/stats', {
                    mode: 'cors'
                }),
                fetch('https://lametrobuspingapi.catenarymaps.org/anomalies?min_rank=90', {
                    mode: 'cors'
                })
            ]);

            if (statsRes.ok) stats = await statsRes.json();
            if (anomaliesRes.ok) anomalies = await anomaliesRes.json();
        } catch (e) {
            error = String(e);
        }
    }

    onMount(() => {
        fetchData();
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
                                        data: bus.history.map(r => ({ x: r.end_of_interval, y: r.latency })), 
                                        color: '#ef4444' 
                                    },
                                    { 
                                        label: 'Interval', 
                                        data: bus.history.map(r => ({ x: r.end_of_interval, y: r.interval })), 
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

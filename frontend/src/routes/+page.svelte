<script lang="ts">
    import { onMount } from 'svelte';
    import LatencyChart from '$lib/components/LatencyChart.svelte';

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
                fetch('/stats'),
                fetch('/anomalies?min_rank=90')
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

    // Helper to extract points
    function toPoints(data: SystemStats[], selector: (s: SystemStats) => number) {
        return data.map(s => ({ x: s.timestamp, y: selector(s) }));
    }

    // Master Charts Data
    let latencyDatasets = $derived([
        { label: 'P99', data: toPoints(stats, s => s.latency_stats.p99), color: '#ef4444' }, // Red
        { label: 'P90', data: toPoints(stats, s => s.latency_stats.p90), color: '#f59e0b' }, // Orange
        { label: 'P50', data: toPoints(stats, s => s.latency_stats.p50), color: '#10b981' }, // Green
    ]);

    let intervalDatasets = $derived([
        { label: 'P99', data: toPoints(stats, s => s.interval_stats.p99), color: '#8b5cf6' }, // Purple
        { label: 'P90', data: toPoints(stats, s => s.interval_stats.p90), color: '#6366f1' }, // Indigo
        { label: 'P50', data: toPoints(stats, s => s.interval_stats.p50), color: '#3b82f6' }, // Blue
    ]);

</script>

<div class="p-8 bg-gray-50 min-h-screen font-sans">
    <h1 class="text-3xl font-bold mb-8">Bus Latency Dashboard</h1>

    {#if error}
        <div class="bg-red-100 p-4 mb-4 rounded text-red-700">{error}</div>
    {/if}

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-12">
        <!-- Master Chart: Latency -->
        <div class="bg-white p-6 rounded shadow">
            <h2 class="text-xl font-semibold mb-2">System Data Latency (Age)</h2>
            <div class="text-sm text-gray-500 mb-4">Seconds behind real-time</div>
            <LatencyChart datasets={latencyDatasets} height="h-64" showLegend={true} />
        </div>

        <!-- Master Chart: Interval -->
        <div class="bg-white p-6 rounded shadow">
            <h2 class="text-xl font-semibold mb-2">System Update Interval (Ping)</h2>
            <div class="text-sm text-gray-500 mb-4">Seconds between updates</div>
            <LatencyChart datasets={intervalDatasets} height="h-64" showLegend={true} />
        </div>
    </div>

    <div>
        <h2 class="text-xl font-semibold mb-4">Top Anomalous Buses (>P90)</h2>
        <div class="space-y-4">
            {#each anomalies as bus (bus.bus_id)}
                <div class="bg-white p-4 rounded shadow grid grid-cols-1 md:grid-cols-[150px_1fr] gap-4 items-center">
                    <div>
                        <div class="text-lg font-bold text-gray-900">{bus.bus_id}</div>
                        <div class="text-sm text-gray-500">Score: <span class="font-mono">{bus.score}</span></div>
                        <div class="text-xs text-gray-400 mt-1">{bus.history.length} data points</div>
                    </div>
                    
                    <div class="h-40 w-full">
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
                        />
                    </div>
                </div>
            {/each}
        </div>
    </div>
</div>

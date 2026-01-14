<script lang="ts">
    import LatencyChart from './LatencyChart.svelte';

    type Percentiles = {
        p0: number;
        p25: number;
        p50: number;
        p75: number;
        p80: number;
        p85: number;
        p90: number;
        p95: number;
        p98: number;
        p99: number;
        p99_5: number;
        p99_9: number;
    };

    type SystemStats = {
        timestamp: number;
        interval_stats: Percentiles;
        latency_stats: Percentiles;
        sample_count: number;
    };

    let { 
        stats = [],
        minTime = null,
        maxTime = null,
        hoveredTime = null,
        onZoom,
        onHover
    } = $props<{
        stats: SystemStats[];
        minTime: number | null;
        maxTime: number | null;
        hoveredTime: number | null;
        onZoom: (min: number, max: number) => void;
        onHover: (time: number | null) => void;
    }>();

    // Helper to extract points
    function toPoints(data: SystemStats[], selector: (s: SystemStats) => number) {
        if (!data || !Array.isArray(data)) return [];
        return data.map(s => ({ x: Number(s.timestamp), y: Number(selector(s)) }));
    }

    // Master Charts Data
    let latencyDatasets = $derived([
        { label: 'P99', data: toPoints(stats, s => s.latency_stats.p99), color: '#ef4444' }, // Red
        { label: 'P98', data: toPoints(stats, s => s.latency_stats.p98), color: '#f43f5e' }, // Rose
        { label: 'P95', data: toPoints(stats, s => s.latency_stats.p95), color: '#f97316' }, // Orange
        { label: 'P90', data: toPoints(stats, s => s.latency_stats.p90), color: '#f59e0b' }, // Amber
        { label: 'P50', data: toPoints(stats, s => s.latency_stats.p50), color: '#10b981' }, // Emerald
    ]);

    let intervalDatasets = $derived([
        { label: 'P99', data: toPoints(stats, s => s.interval_stats.p99), color: '#8b5cf6' }, // Violet
        { label: 'P98', data: toPoints(stats, s => s.interval_stats.p98), color: '#a855f7' }, // Purple
        { label: 'P95', data: toPoints(stats, s => s.interval_stats.p95), color: '#d946ef' }, // Fuchsia
        { label: 'P90', data: toPoints(stats, s => s.interval_stats.p90), color: '#6366f1' }, // Indigo
        { label: 'P50', data: toPoints(stats, s => s.interval_stats.p50), color: '#3b82f6' }, // Blue
    ]);
</script>

<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-12">
    <!-- Master Chart: Latency -->
    <div class="bg-white p-6 rounded shadow overflow-hidden">
        <h2 class="text-xl font-semibold mb-2">Bus Latency (Age)</h2>
        <div class="text-sm text-gray-500 mb-4">Time it took to reach swiftly's system</div>
        <LatencyChart 
            datasets={latencyDatasets} 
            height="h-64" 
            showLegend={true} 
            min={minTime} 
            max={maxTime} 
            onZoom={onZoom}
            hoveredTime={hoveredTime}
            onHover={onHover}
        />
    </div>

    <!-- Master Chart: Interval -->
    <div class="bg-white p-6 rounded shadow overflow-hidden">
        <h2 class="text-xl font-semibold mb-2">Interval Distribution</h2>
        <div class="text-sm text-gray-500 mb-4">Seconds between updates for each bus</div>
        <LatencyChart 
            datasets={intervalDatasets} 
            height="h-64" 
            showLegend={true} 
            min={minTime} 
            max={maxTime} 
            onZoom={onZoom}
            hoveredTime={hoveredTime}
            onHover={onHover}
        />
    </div>
</div>

<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import Chart from 'chart.js/auto';

    interface Dataset {
        label: string;
        data: { x: number; y: number }[];
        color: string;
        borderColor?: string;
        backgroundColor?: string;
        fill?: boolean;
    }

    let { datasets, title, height = "h-32", showLegend = false } = $props<{
        datasets: Dataset[];
        title?: string;
        height?: string;
        showLegend?: boolean;
    }>();

    let canvas: HTMLCanvasElement;
    let chart: Chart;

    function updateChart() {
        if (!chart) return;
        
        chart.data.datasets = datasets.map(ds => ({
            label: ds.label,
            data: ds.data,
            borderColor: ds.color,
            backgroundColor: ds.backgroundColor, // for fill
            borderWidth: 1.5,
            tension: 0.1,
            pointRadius: 0,
            fill: ds.fill
        }));
        chart.update('none');
    }

    onMount(() => {
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        chart = new Chart(ctx, {
            type: 'line',
            data: {
                datasets: []
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                interaction: {
                    mode: 'index',
                    intersect: false,
                },
                scales: {
                    x: {
                        type: 'linear',
                        display: false, // hide x axis labels for cleanliness
                    },
                    y: {
                        beginAtZero: true
                    }
                },
                plugins: {
                    legend: {
                        display: showLegend,
                        position: 'top',
                        labels: {
                            boxWidth: 10,
                            font: {
                                size: 10
                            }
                        }
                    },
                    tooltip: {
                        enabled: true
                    },
                    title: {
                        display: !!title,
                        text: title || ''
                    }
                }
            }
        });
        
        updateChart();
    });

    $effect(() => {
        if(chart && datasets) {
            updateChart();
        }
    });
    
    onDestroy(() => {
        if (chart) chart.destroy();
    });
</script>

<div class="{height} w-full relative">
    <canvas bind:this={canvas}></canvas>
</div>

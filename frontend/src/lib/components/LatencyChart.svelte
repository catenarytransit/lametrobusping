<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import Chart from 'chart.js/auto';
    import { getRelativePosition } from 'chart.js/helpers';

    interface Dataset {
        label: string;
        data: { x: number; y: number }[];
        color: string;
        borderColor?: string;
        backgroundColor?: string;
        fill?: boolean;
    }

    let { 
        datasets, 
        title, 
        height = "h-32", 
        showLegend = false,
        min = $bindable(null),
        max = $bindable(null),
        onZoom = () => {},
        hoveredTime = null,
        onHover = () => {}
    } = $props<{
        datasets: Dataset[];
        title?: string;
        height?: string;
        showLegend?: boolean;
        min?: number | null;
        max?: number | null;
        hoveredTime?: number | null;
        onZoom?: (min: number, max: number) => void;
        onHover?: (time: number | null) => void;
    }>();

    let canvas: HTMLCanvasElement;
    let chart: Chart;

    // Plugin to draw vertical line
    const verticalLinePlugin = {
        id: 'verticalLine',
        defaults: {
            color: '#9ca3af',
            width: 1,
            dash: [5, 5]
        },
        afterDraw: (chart: Chart, args: any, options: any) => {
            if (options.xValue === null || options.xValue === undefined) return;
            
            const ctx = chart.ctx;
            const xAxis = chart.scales.x;
            const yAxis = chart.scales.y;
            
            const x = xAxis.getPixelForValue(options.xValue);
            
            // Only draw if within chart area (horizontally)
            if (x < chart.chartArea.left || x > chart.chartArea.right) return;

            ctx.save();
            ctx.beginPath();
            ctx.moveTo(x, yAxis.top);
            ctx.lineTo(x, yAxis.bottom);
            ctx.lineWidth = options.width;
            ctx.strokeStyle = options.color;
            ctx.setLineDash(options.dash);
            ctx.stroke();
            ctx.restore();
        }
    };

    function updateChart() {
        if (!chart) return;
        
        // Find min and max from all datasets
        let minX = Infinity;
        let maxX = -Infinity;
        let hasData = false;

        datasets.forEach(ds => {
            ds.data.forEach(p => {
                if (p.x < minX) minX = p.x;
                if (p.x > maxX) maxX = p.x;
                hasData = true;
            });
        });

        if (hasData) {
            // Align to hourly boundaries
            // 3600 seconds = 1 hour
            const stepSize = 3600;
            const alignedMin = Math.floor(minX / stepSize) * stepSize;
            const alignedMax = Math.ceil(maxX / stepSize) * stepSize;

            if (chart.options.scales?.x) {
                // If external min/max are provided, use them. Otherwise use aligned auto range.
                if (min !== null && max !== null) {
                    chart.options.scales.x.min = min;
                    chart.options.scales.x.max = max;
                } else {
                    chart.options.scales.x.min = alignedMin;
                    chart.options.scales.x.max = alignedMax;
                }

                if (chart.options.scales.x.ticks) {
                     chart.options.scales.x.ticks.stepSize = stepSize;
                     delete chart.options.scales.x.ticks.maxTicksLimit;
                }
            }
        }

        const rawDatasets = $state.snapshot(datasets);

        // Preserve hidden status from current datasets
        const hiddenStatus = new Map<string, boolean>();
        if (chart.data && chart.data.datasets) {
             chart.data.datasets.forEach((ds, index) => {
                if (ds.label) {
                    const isVisible = chart.isDatasetVisible(index);
                    hiddenStatus.set(ds.label, !isVisible);
                }
            });
        }

        chart.data.datasets = rawDatasets.map(ds => ({
            label: ds.label,
            data: ds.data,
            borderColor: ds.color,
            backgroundColor: ds.backgroundColor, // for fill
            borderWidth: 1.5,
            tension: 0.1,
            pointRadius: 0,
            fill: ds.fill,
            hidden: hiddenStatus.has(ds.label) ? hiddenStatus.get(ds.label) : undefined
        }));

        // Update plugin options
        if (chart.options.plugins?.verticalLine) {
            chart.options.plugins.verticalLine.xValue = hoveredTime;
        }

        // Update legend and axis visibility
        if (chart.options.plugins?.legend) {
            chart.options.plugins.legend.display = showLegend;
        }
        if (chart.options.scales?.x) {
            chart.options.scales.x.display = showLegend;
        }

        chart.update('none');
        chart.resize();
    }

    onMount(async () => {
        try {
            const { default: zoomPlugin } = await import('chartjs-plugin-zoom');
            Chart.register(zoomPlugin);
        } catch (err) {
            console.error('LatencyChart: Failed to import/register zoom plugin', err);
        }

        const ctx = canvas.getContext('2d');
        if (!ctx) {
            console.error('Canvas context not found');
            return;
        }

        chart = new Chart(ctx, {
            type: 'line',
            data: {
                datasets: []
            },
            plugins: [verticalLinePlugin],
            options: {
                responsive: true,
                maintainAspectRatio: false,
                animation: false, // Disable animations as requested
                interaction: {
                    mode: 'index',
                    intersect: false,
                },
                onHover: (event: any, elements, chart) => {
                    const canvasPosition = getRelativePosition(event, chart);
                    // Check if mouse is inside chart area
                    if(canvasPosition.x >= chart.chartArea.left && canvasPosition.x <= chart.chartArea.right &&
                       canvasPosition.y >= chart.chartArea.top && canvasPosition.y <= chart.chartArea.bottom) {
                        const xVal = chart.scales.x.getValueForPixel(canvasPosition.x);
                        onHover(xVal);
                    } else {
                        onHover(null);
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        display: showLegend, // Use showLegend as proxy for "large chart" for now, or add specific prop
                        ticks: {
                            callback: function(value) {
                                const date = new Date(Number(value) * 1000);
                                return new Intl.DateTimeFormat('en-US', {
                                    timeZone: 'America/Los_Angeles',
                                    hour: '2-digit',
                                    minute: '2-digit',
                                    hour12: false
                                }).format(date);
                            },
                            stepSize: 3600 // Default, will be overridden in updateChart if data exists
                        },
                        grid: {
                            // Ensure grid lines are drawn
                            display: true
                        }
                    },
                    y: {
                        beginAtZero: true
                    }
                },
                plugins: {
                    verticalLine: {
                        xValue: hoveredTime
                    },
                    zoom: {
                        zoom: {
                            drag: {
                                enabled: true,
                                backgroundColor: 'rgba(59, 130, 246, 0.2)' // blue-500 with opacity
                            },
                            mode: 'x',
                            onZoom: ({chart}) => {
                                const {min, max} = chart.scales.x;
                                onZoom(min, max);
                            },
                            onZoomComplete: ({chart}) => {
                                const {min, max} = chart.scales.x;
                                onZoom(min, max);
                            }
                        }
                    },
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
                        // Disable built-in tooltip if conflicting? 
                        // But we want tooltips to show values.
                        // We might want to sync tooltips too, but minimal request is "common hover vertical line".
                        callbacks: {
                            title: function(context) {
                                const value = context[0].parsed.x;
                                const date = new Date(value * 1000);
                                return new Intl.DateTimeFormat('en-US', {
                                    timeZone: 'America/Los_Angeles',
                                    month: 'short',
                                    day: 'numeric',
                                    hour: '2-digit',
                                    minute: '2-digit',
                                    second: '2-digit',
                                    hour12: false
                                }).format(date);
                            }
                        }
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
        // Track all dependencies
        const _d = datasets;
        const _m = min;
        const _mx = max;
        const _h = hoveredTime;

        if (chart) {
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

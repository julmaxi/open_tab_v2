<script>
    import { dateTicks, InteractivePlot, Plot, PointsGraph } from "$lib/plot";
    import { onMount } from "svelte";

    export let samples;

    let v = {
        width: 400,
        height: 400,
    }

    let symbolTable = [
        "triangle",
        "square",
        "pentagon",
    ];

    let colorTable = {
        "NonAligned": "rgb(254 249 195)",
        "Government": "rgb(220 252 231)",
        "Ppposition": "rgb(243 232 255",
    }

    let instances = samples.map((sample) => {
        console.log(sample.time, new Date(sample.time))
        return {
            x: new Date(sample.time).getTime(),
            y: sample.total_score,
            symbol: sample.position <= 3 ? symbolTable[sample.position - 1] : "circle",
            color: colorTable[sample.role] || "black"
        };
    });

    console.log(instances)

    let popoverPosition = null;
    let canvas = null;

    onMount(() => {
        let minTime = new Date(2013, 12, 8).getTime();

        // New dataPoints with at least 40 entries, spanning from 2020 through March 2025.
        let dataPoints = [
            // Cluster 1 (January 1, 2020): 4 points (2-hour intervals)
            { x: new Date(2020, 0, 1, 8, 0).getTime(),   y: 30.0 },
            { x: new Date(2020, 0, 1, 10, 0).getTime(),  y: 32.33 },
            { x: new Date(2020, 0, 1, 12, 0).getTime(),  y: 35.25 },
            { x: new Date(2020, 0, 1, 14, 0).getTime(),  y: 33.1 },
            // Cluster 2 (January 15, 2020): 3 points
            { x: new Date(2020, 0, 15, 9, 0).getTime(),  y: 40.0 },
            { x: new Date(2020, 0, 15, 11, 0).getTime(), y: 42.2 },
            { x: new Date(2020, 0, 15, 13, 0).getTime(), y: 41.33 },
            // Cluster 3 (February 5, 2020): 5 points
            { x: new Date(2020, 1, 5, 8, 30).getTime(),  y: 50.25 },
            { x: new Date(2020, 1, 5, 10, 30).getTime(), y: 52.1 },
            { x: new Date(2020, 1, 5, 12, 30).getTime(), y: 55.33 },
            { x: new Date(2020, 1, 5, 14, 30).getTime(), y: 57.2 },
            { x: new Date(2020, 1, 5, 16, 30).getTime(), y: 60.0 },
            // Cluster 4 (May 10, 2021): 4 points
            { x: new Date(2021, 4, 10, 9, 0).getTime(),  y: 45.0 },
            { x: new Date(2021, 4, 10, 11, 0).getTime(), y: 47.33 },
            { x: new Date(2021, 4, 10, 13, 0).getTime(), y: 46.25 },
            { x: new Date(2021, 4, 10, 15, 0).getTime(), y: 48.2 },
            // Cluster 5 (October 5, 2021): 4 points
            { x: new Date(2021, 9, 5, 8, 30).getTime(),  y: 50.0 },
            { x: new Date(2021, 9, 5, 10, 30).getTime(), y: 52.33 },
            { x: new Date(2021, 9, 5, 12, 30).getTime(), y: 51.25 },
            { x: new Date(2021, 9, 5, 14, 30).getTime(), y: 53.1 },
            // Cluster 6 (March 12, 2022): 4 points
            { x: new Date(2022, 2, 12, 9, 0).getTime(),  y: 55.0 },
            { x: new Date(2022, 2, 12, 11, 0).getTime(), y: 57.2 },
            { x: new Date(2022, 2, 12, 13, 0).getTime(), y: 56.33 },
            { x: new Date(2022, 2, 12, 15, 0).getTime(), y: 58.0 },
            // Cluster 7 (August 20, 2022): 4 points
            { x: new Date(2022, 7, 20, 8, 30).getTime(),  y: 60.0 },
            { x: new Date(2022, 7, 20, 10, 30).getTime(), y: 62.33 },
            { x: new Date(2022, 7, 20, 12, 30).getTime(), y: 61.25 },
            { x: new Date(2022, 7, 20, 14, 30).getTime(), y: 63.1 },
            // Cluster 8 (January 10, 2023): 5 points
            { x: new Date(2023, 0, 10, 8, 0).getTime(),   y: 45.0 },
            { x: new Date(2023, 0, 10, 10, 0).getTime(),  y: 47.33 },
            { x: new Date(2023, 0, 10, 12, 0).getTime(),  y: 49.25 },
            { x: new Date(2023, 0, 10, 14, 0).getTime(),  y: 48.1 },
            { x: new Date(2023, 0, 10, 16, 0).getTime(),  y: 50.2 },
            // Cluster 9 (July 4, 2023): 5 points
            { x: new Date(2023, 6, 4, 9, 0).getTime(),   y: 55.0 },
            { x: new Date(2023, 6, 4, 11, 0).getTime(),  y: 57.33 },
            { x: new Date(2023, 6, 4, 13, 0).getTime(),  y: 56.25 },
            { x: new Date(2023, 6, 4, 15, 0).getTime(),  y: 58.2 },
            { x: new Date(2023, 6, 4, 17, 0).getTime(),  y: 59.1 },
            // Cluster 10 (March 5, 2024): 4 points
            { x: new Date(2024, 2, 5, 8, 30).getTime(),  y: 65.0, symbol: "pentagon", color: "red" },
            { x: new Date(2024, 2, 5, 10, 30).getTime(), y: 67.33, symbol: "triangle" },
            { x: new Date(2024, 2, 5, 12, 30).getTime(), y: 66.25 },
            { x: new Date(2024, 2, 5, 14, 30).getTime(), y: 68.1 },
            // Cluster 11 (September 10, 2024): 4 points
            { x: new Date(2024, 8, 10, 9, 0).getTime(),  y: 60.0 },
            { x: new Date(2024, 8, 10, 11, 0).getTime(), y: 62.2 },
            { x: new Date(2024, 8, 10, 13, 0).getTime(), y: 61.33 },
            { x: new Date(2024, 8, 10, 15, 0).getTime(), y: 63.1 },
            // Cluster 12 (March 1, 2025): 4 points
            { x: new Date(2025, 2, 1, 8, 0).getTime(),   y: 65.0 },
            { x: new Date(2025, 2, 1, 10, 0).getTime(),  y: 67.33 },
            { x: new Date(2025, 2, 1, 12, 0).getTime(),  y: 66.25 },
            { x: new Date(2025, 2, 1, 14, 0).getTime(),  y: 68.2 },
        ];

        let minScore = Math.min(...instances.map(instance => instance.y));
        let maxScore = Math.max(...instances.map(instance => instance.y));
        let y = Math.min(30, minScore - 1);
        let maxY = Math.max(70, Math.min(maxScore + 1, 100));

        let interactive = new InteractivePlot(
            canvas,
            {
                x: instances[0].x - 24 * 60 * 60 * 1000,
                y,
                width: instances[instances.length - 1].x - instances[0].x + 2 * 24 * 60 * 60 * 1000,
                height: maxY - y,
            },
            {
                x: minTime,
                y: 0,
                width: new Date().getTime() - minTime + 24 * 60 * 60 * 1000,
                height: 100,
            },
            {
                lockYZoom: true,
                lockYPan: true,
                xTicks: dateTicks()
            }
        );
                
        interactive.addChild(
            new PointsGraph(
                instances
            )
        );

        interactive.onClickPoint = (event) => {
            popoverPosition = {
                x: event.position.x,
                y: event.position.y,
            };
            selectedSample = samples[event.index];
        }
    });

    let selectedSample = null;

    function convertOrdinal(n) {
        const s = ["th", "st", "nd", "rd"];
        const v = n % 100;
        return n + (s[(v - 20) % 10] || s[v] || s[0]);
    }

    export function convertPositionRole(position, role) {
        if (role != "NonAligned") {
            return `${convertOrdinal(position)} Speaker of the ${role}`;
        }
        else {
            return `${convertOrdinal(position)} Non-Aligned Speaker`;
        }
    }
</script>

<style>
    .popover {
        position: absolute;
        background: linear-gradient(135deg, #ffffff, #f9f9f9);
        padding: 10px;
        z-index: 10;
        border: 1px solid #ddd;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
        transition: transform 0.2s ease, opacity 0.2s ease;
    }
    
    .role {
        font-size: 0.8rem;
        color: #555;
    }

    .score {
        font-size: 1rem;
        font-weight: bold;
        color: #333;
    }

    canvas {
        width: 100%;
        max-width: 400px;
    }

    .caption {
        text-align: center;
        width: 100%;
        font-weight: bold;
    }
</style>


<div style="position: relative;" on:click={() => popoverPosition = null}>
    <div>
    <canvas
        width="800"
        height="800"
        bind:this={canvas}
    >
    </canvas>
    <div class="caption">
        Timeline
    </div>
    </div>

    {#if popoverPosition}
        <div
            style="position: absolute; top: {popoverPosition.y}px; left: {popoverPosition.x}px;"
            class="popover"
        >
            <h3>
                {new Date(selectedSample.time).toLocaleDateString(
                    undefined,
                    {
                        year: "numeric",
                        month: "2-digit",
                        day: "2-digit",
                        hour: "2-digit",
                        minute: "2-digit",
                    }
                )}
            </h3>

            <p class="score">{selectedSample.total_score.toFixed(1)} Points</p>
            <p class="role">{convertPositionRole(selectedSample.position, selectedSample.role)}</p>
        </div>
    {/if}
</div>
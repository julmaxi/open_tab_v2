<script>
    import { KDEPlot, Plot } from "$lib/plot";
    import { onMount } from "svelte";

    export let samples = [];

    let canvas = null;

    onMount(() => {
        let plot = new Plot(
            {
                x: 0,
                y: 0,
                width: 400,
                height: 400,
            },
            {
                x: 20,
                y: 0,
                width: 70 - 20,
                height: 1.0 + 0.1
            }
        );

        plot.addChild(
            new KDEPlot(
                samples.map((sample) => sample.total_score),
            )
        );

        plot.render(canvas?.getContext("2d"));
    })
</script>

<style>
    .caption {
        text-align: center;
        width: 100%;
        font-weight: bold;
    }
</style>

<div>
<canvas
    width="800"
    height="800"
    style="width: 400px; height: 400px"
    bind:this={canvas}
>
</canvas>
<div class="caption">
    Distribution
</div>
</div>
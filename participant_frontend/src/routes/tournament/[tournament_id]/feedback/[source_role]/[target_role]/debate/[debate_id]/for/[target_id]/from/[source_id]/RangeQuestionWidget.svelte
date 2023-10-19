<script>
    export let config;
    export let name;

    let midPoint = (config.min + config.max) / 2;

    let value = midPoint;
</script>

<div class="p-4 md:p0">
    <input type="range" min={config.min} max={config.max} step={config.step} class="w-full" bind:value />
    <div class="flex">
        <span class="flex-1">{config.labels[0][1]}</span>
        <span class="flex-1 text-right">{config.labels[1][1]}</span>
    </div>
    <div class="flex justify-center items-center">
        <input type="number" name={name} min={config.min} max={config.max} step={config.step} bind:value />
    </div>
</div>
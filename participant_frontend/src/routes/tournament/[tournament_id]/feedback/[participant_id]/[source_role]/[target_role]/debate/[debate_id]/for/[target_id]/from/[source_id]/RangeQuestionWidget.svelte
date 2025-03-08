<script>
    export let config;
    export let name;
    export let initialValue = null;

    let value = initialValue;
    let midPoint = (config.min + config.max) / 2;

    let relativeScore = 0.0;

    $: switch (config.orientation) {
        case "MeanIsGood":
            relativeScore = 1.0 - Math.abs(value - midPoint) / (config.max - midPoint);
            break;
        case "HighIsGood":
            relativeScore = (value - config.min) / (config.max - config.min);
            break;
        case "LowIsGood":
            relativeScore = (config.max - value) / (config.max - config.min);
            break;
    }

    let labels = config.labels;
    labels.sort((a, b) => a[0] - b[0]);

    $: hue = relativeScore * 120;
</script>


<style>
    input[type=range] {
        width: 100%;
        margin: 13.8px 0;
    }

    input[type=number] {
        text-align: center;
        padding: 0.25rem;
        border-radius: 0.25rem;

        font-weight: 800;
    }

    .clear {
        background-color: #f8f9fa;
        border: 1px solid #ccc;
        border-radius: 0.25rem;
        padding: 0.25rem;
        margin-top: 0.5rem;
    }
</style>

<div class="p-4 md:p0">
    <input type="range" min={config.min} max={config.max} step={config.step} class="w-full" on:input={
        (e) => {
            value = parseFloat(e.target.value);
        }
    } value={
        value != null ? value : null
    } />
    <div class="flex">
        <span class="flex-1">{labels[0][1]}</span>
        <span class="flex-1 text-right">{labels[1][1]}</span>
    </div>
    <div class="flex justify-center items-center">
        <input style={`color: hsl(${hue} 60% 45%)`} type="number" name={name} min={config.min} max={config.max} step={config.step} placeholder="-" value={
            value
        } on:change={
            (e) => {
                value = parseFloat(e.target.value);
                if (isNaN(value)) {
                    value = null;
                }
            }
        } />
    </div>
</div>
<script>
    export let cardType = "info";

    let cardClass = "card";

    switch (cardType) {
        case "error":
            cardClass += " error-card";
            break;
        case "info":
            cardClass += " info-card";
            break;
        case "disabled":
            cardClass += " disabled-card";
            break;
        default:
            cardClass += " info-card";
            break;
    }
</script>

<style>
    .card {
        border-radius: 0.25rem;
        margin: 0.25rem;
        background-color: white;
        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);
    }

    .error-card {
        background: linear-gradient(0deg, rgba(184,6,6,1) 0%, rgba(177,0,0,1) 100%);
        color: white;
    }

    .disabled-card {
        background-color: rgb(237, 237, 237);
    }
</style>


<div class={cardClass}>
    <div class="card-content">
        <slot name="content" />
    </div>
    
    <div>
        <slot name="footer" />
    </div>
</div>
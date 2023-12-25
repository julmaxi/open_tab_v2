<script>
    import BellAnimation from './BellAnimation.svelte';
</script>

<style>
    .modal {
        width: 100vw;
        height: 100vh;
        background-color: rgba(0, 0, 0, 0.5);
        position: fixed;
        z-index: 1000;
    }
</style>


<div class="modal">
    <div class="flex w-full h-full items-center justify-center">
        <div style="width: 50%; max-width: 100px;">
        <BellAnimation />
        <p class="text-white font-bold">Submitting...</p>
        </div>
    </div>
</div>

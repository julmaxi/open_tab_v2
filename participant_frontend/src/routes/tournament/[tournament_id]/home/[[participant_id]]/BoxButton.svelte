<script>
    export let href;
    export let label;
</script>
<style>
        a {
        width: 100%;
        display: flex;
        flex-direction: row;
        padding-top: 7px;
        padding-bottom: 7px;
        padding-left: 8px;
        border-top: 1px solid #ccc;
        font-weight: bold;
        align-items: center;
        line-height: 1rem;
    }

    a > svg {
        display: inline-block;
    }
    
    a > div {
        flex: 1;
        padding-left: 2px;
    }
</style>
<a href={href}>
    <svg width="100%" height="100%" viewBox="0 0 53 24" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xml:space="preserve" xmlns:serif="http://www.serif.com/" style="fill-rule:evenodd;clip-rule:evenodd;stroke-linejoin:round;stroke-miterlimit:2;width: 16px;">
        <g transform="matrix(0.659646,0,0,0.303719,-91.918,-84.3739)">
            <rect x="139.344" y="300" width="57.498" height="32.989" style="fill:currentColor;"/>
        </g>
        <g transform="matrix(4.65291e-17,0.762781,-0.759878,4.67069e-17,264.278,-147.067)">
            <path d="M207.073,278.666C207.319,278.273 207.748,278.035 208.209,278.035C208.671,278.035 209.1,278.273 209.345,278.666C212.326,283.441 220.356,296.301 223.419,301.206C223.662,301.596 223.676,302.088 223.454,302.49C223.232,302.893 222.81,303.143 222.351,303.143C216.335,303.143 200.071,303.143 194.061,303.143C193.604,303.143 193.183,302.894 192.962,302.492C192.74,302.091 192.754,301.6 192.996,301.211C196.057,296.31 204.091,283.442 207.073,278.666Z" style="fill:currentColor;"/>
        </g>
    </svg>
    <div>{label}</div>
</a>

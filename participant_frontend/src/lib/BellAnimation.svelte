<script>
    export let color = "#FFFFFF";
</script>


<style>
.bell {
    width: 100%;
    height: 100%;
}
.bell-body {
    transform-box: fill-box;
    transform-origin: center;
    animation-iteration-count: infinite;
    animation-direction: alternate;
    animation-timing-function: linear;
    animation-duration: 0.5s;
    animation-name: swingbell;
}

@keyframes swingbell {
    0% {
        transform: rotate(20deg);
    }

    100% {
        transform: rotate(-20deg);
    }
}

.bell-clapper {
    transform-box: fill-box;
    transform-origin: center;

    animation-iteration-count: infinite;
    animation-direction: alternate;
    animation-timing-function: linear;
    animation-name: clapbell;
    animation-duration: 0.5s;
}


@keyframes clapbell {
    0% { transform: translate(0px, -40px) rotate(12deg) translate(0px, 40px) }
    100% { transform: translate(0px, -40px) rotate(-12deg) translate(0px, 40px) }
}
</style>

<svg class="bell" viewBox="-30 0 162 202" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xml:space="preserve" xmlns:serif="http://www.serif.com/" style={`fill-rule:evenodd;clip-rule:evenodd;stroke-linejoin:round;stroke-miterlimit:2;fill:${color};`}>
    <g transform="matrix(1,0,0,1,-236.423,-111.226)">
    <g class="bell-body" transform="rotate(20)">
  
        <g transform="matrix(1,0,0,1.02996,1.80703,13.7726)">
            <path d="M294.303,99.005C294.303,96.584 292.279,94.619 289.785,94.619L280.75,94.619C278.257,94.619 276.233,96.584 276.233,99.005L276.233,169.906C276.233,172.327 278.257,174.292 280.75,174.292L289.785,174.292C292.279,174.292 294.303,172.327 294.303,169.906L294.303,99.005Z"/>
        </g>
        <g transform="matrix(1,0,0,0.680826,0,69.6817)">
            <rect x="243.049" y="218.319" width="88.053" height="81.681"/>
        </g>
        <g transform="matrix(1.13959,0,0,1.13959,-3.17842,-6.73032)">
            <circle cx="254.7" cy="197.483" r="38.633"/>
        </g>
        <g transform="matrix(1,0,0,1,45.8346,0.122012)">
            <path d="M291.892,270.276C291.892,268.328 290.31,266.745 288.361,266.745L194.119,266.745C192.17,266.745 190.588,268.328 190.588,270.276L190.588,277.338C190.588,279.287 192.17,280.869 194.119,280.869L288.361,280.869C290.31,280.869 291.892,279.287 291.892,277.338L291.892,270.276Z"/>
        </g>
    </g>
    <g class="bell-clapper">
        <g>
            <g transform="matrix(1,0,0,0.696105,-7.22813,76.1808)">
                <rect x="287.075" y="300" width="14.456" height="24.02"/>
            </g>
            <g transform="matrix(1,0,0,1,-3.61407,-30.9831)">
                <circle cx="290.689" cy="332.716" r="10.842"/>
            </g>
        </g>
    </g>
</g>
</svg>
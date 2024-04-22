const FaSvgIcon = ({ faIcon, ...rest }) => {
    const { width, height, svgPathData } = faIcon;
    return (
        <svg {...rest} viewBox={`0 0 ${width} ${height}`} fill="currentColor">
            <path d={svgPathData}></path>
        </svg>
    );
};

export {
    FaSvgIcon
}

import React from "react";
import LoadingSpinner from "../../misc/LoadingSpinner";
import SimpleLineChart from "../../charts/SimpleLineChart";

function DNSStatisticsChart(props) {

    if (!props.statistics) {
        return <LoadingSpinner />
    }

    return <SimpleLineChart
        height={150}
        lineWidth={1}
        data={formatData(props.statistics.buckets, props.attribute)}
    />

}

function formatData(data, attribute) {
    const result = {};

    Object.keys(data).sort().forEach(function (key) {
        result[key] = data[key][attribute];
    })

    return result;
}

export default DNSStatisticsChart;
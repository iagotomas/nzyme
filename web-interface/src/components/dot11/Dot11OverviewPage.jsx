import React, {useContext, useEffect, useState} from "react";
import Dot11Service from "../../services/Dot11Service";
import {TapContext} from "../../App";
import LoadingSpinner from "../misc/LoadingSpinner";
import {disableTapSelector, enableTapSelector} from "../misc/TapSelector";
import AutoRefreshSelector from "../misc/AutoRefreshSelector";
import BSSIDAndSSIDChart from "./bssids/BSSIDAndSSIDChart";
import ClientHistogram from "./clients/ClientHistogram";
import ApiRoutes from "../../util/ApiRoutes";

const dot11Service = new Dot11Service();

const loadData = function(taps, setBSSIDs, setLastUpdated, setHistograms) {
    dot11Service.findAllBSSIDs(15, taps, setBSSIDs);
    dot11Service.getClientHistograms(taps, setHistograms);
    setLastUpdated(new Date());
}

function Dot11OverviewPage() {

    const tapContext = useContext(TapContext);
    const selectedTaps = tapContext.taps;

    const [isAutoRefresh, setIsAutoRefresh] = useState(true);
    const [lastUpdated, setLastUpdated] = useState(null);

    const [bssids, setBSSIDs] = useState(null);
    const [histograms, setHistograms] = useState(null);


    useEffect(() => {
        loadData(selectedTaps, setBSSIDs, setLastUpdated, setHistograms);

        const timer = setInterval(() => {
            if (isAutoRefresh) {
                loadData(selectedTaps, setBSSIDs, setLastUpdated, setHistograms);
            }
        }, 15000);

        return () => clearInterval(timer);
    }, [isAutoRefresh, selectedTaps])

    useEffect(() => {
        enableTapSelector(tapContext);

        return () => {
            disableTapSelector(tapContext);
        }
    }, [tapContext]);

    if (!bssids) {
        return <LoadingSpinner />
    }

    return (
        <React.Fragment>
            <div className="row">
                <div className="col-md-12">
                    <div className="float-end">
                        <AutoRefreshSelector isAutoRefresh={isAutoRefresh}
                                             setIsAutoRefresh={setIsAutoRefresh}
                                             lastUpdated={lastUpdated} />
                    </div>
                </div>
            </div>

            <div className="row">
                <div className="card">
                    <div className="card-body">
                        <div className="col-md-12">
                            <h1><a href={ApiRoutes.DOT11.NETWORKS.BSSIDS}>Access points</a></h1>
                        </div>
                        <div className="row mt-3">
                            <div className="col-md-12">
                                <div className="row">
                                    <div className="col-md-6">
                                        <h3 className="mb-0">Active BSSIDs</h3>
                                        <BSSIDAndSSIDChart parameter="bssid_count" />
                                    </div>
                                    <div className="col-md-6">
                                        <h3 className="mb-0">Active SSIDs</h3>
                                        <BSSIDAndSSIDChart parameter="ssid_count" />
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <div className="row">
                <div className="row mt-3">

                </div>
            </div>
            <div className="row">
                <div className="card">
                    <div className="card-body">
                        <div className="col-md-12">
                            <h1><a href={ApiRoutes.DOT11.CLIENTS.INDEX}>Clients</a></h1>
                        </div>
                        <div className="row mt-3">
                            <div className="col-md-6">
                                <h3>Connected Clients</h3>

                                <ClientHistogram param="connected" histograms={histograms} />
                            </div>

                            <div className="col-md-6">
                                <h3>Disconnected Clients</h3>

                                <ClientHistogram param="disconnected" histograms={histograms} />
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </React.Fragment>
    )


}

export default Dot11OverviewPage;
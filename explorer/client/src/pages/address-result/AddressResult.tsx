// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useState, useEffect, useContext } from 'react';
import { useParams } from 'react-router-dom';

import ErrorResult from '../../components/error-result/ErrorResult';
import Longtext from '../../components/longtext/Longtext';
import OwnedObjects from '../../components/ownedobjects/OwnedObjects';
import { NetworkContext } from '../../context';
import theme from '../../styles/theme.module.css';
import { DefaultRpcClient as rpc } from '../../utils/api/DefaultRpcClient';
import { IS_STATIC_ENV } from '../../utils/envUtil';

type DataType = {
    id: string;
    objects: ResponseType;
    loadState?: 'loaded' | 'pending' | 'fail';
};

type ResponseType = {
    objectId: string;
}[];

function instanceOfDataType(object: any): object is DataType {
    return object !== undefined && ['id', 'objects'].every((x) => x in object);
}
const DATATYPE_DEFAULT = {
    to: [],
    from: [],
    loadState: 'pending',
};

function TxForAddress({ id }: { id: string }) {
    const [showData, setData] = useState(DATATYPE_DEFAULT);
    const [network] = useContext(NetworkContext);
    const deduplicate = (results: [string, string][]) =>
        results
            .map((result) => result[1])
            .filter((value, index, self) => self.indexOf(value) === index);

    useEffect(() => {
        rpc(network)
            .getTransactionsForAddress(id)
            .then((data) =>
                setData({
                    ...(data as typeof DATATYPE_DEFAULT),
                    loadState: 'loaded',
                })
            )
            .catch((error) => {
                console.log(error);
                setData({ ...DATATYPE_DEFAULT, loadState: 'fail' });
            });
    }, [id, network]);

    if (showData.loadState === 'pending') {
        return <div>Loading ...</div>;
    }

    if (showData.loadState === 'loaded') {
        return (
            <>
                <div>
                    <div>Transactions Sent</div>
                    <div>
                        {deduplicate(showData.from).map((x, index) => (
                            <div key={`from-${index}`}>
                                <Longtext
                                    text={x}
                                    category="transactions"
                                    isLink={true}
                                />
                            </div>
                        ))}
                    </div>
                </div>
                <div>
                    <div>Transactions Received</div>
                    <div>
                        {deduplicate(showData.to).map((x, index) => (
                            <div key={`to-${index}`}>
                                <Longtext
                                    text={x}
                                    category="transactions"
                                    isLink={true}
                                />
                            </div>
                        ))}
                    </div>
                </div>
            </>
        );
    }
    return (
        <ErrorResult
            id={id}
            errorMsg="Transactions could not be extracted on the following specified transaction ID"
        />
    );
}

function AddressResult() {
    const { id: addressID } = useParams();

    if (addressID !== undefined) {
        return (
            <div className={theme.textresults} id="textResults">
                <div>
                    <div>Address</div>
                    <div id="addressID">
                        <Longtext
                            text={addressID}
                            category="addresses"
                            isLink={false}
                        />
                    </div>
                </div>
                {!IS_STATIC_ENV && <TxForAddress id={addressID} />}
                <div>
                    <div>Owned Objects</div>
                    <div>{<OwnedObjects id={addressID} />}</div>
                </div>
            </div>
        );
    } else {
        return <ErrorResult id={addressID} errorMsg={'Something went wrong'} />;
    }
}

export default AddressResult;
export { instanceOfDataType };



import { Typography } from '@mui/material';
import { useEffect, useMemo, useState } from 'react';
import ApiUrlFooter from '../components/ApiUrlFooter';
import { JsonEditor } from '../components/JsonEditor';
import { ViewUnderAppBarBox, FullBoxContainer, QBreadcrumbs } from '../components/LayoutUtils';
import Loader from '../components/Loader';
import ErrorResponseDisplay from '../components/ResponseErrorDisplay';
import { Client } from '../services/client';
import { Cluster, ResponseError } from '../utils/models';


function ClusterView() {
  const [loading, setLoading] = useState(false);
  const [cluster, setCluster] = useState<null | Cluster>(null);
  const [responseError, setResponseError] = useState<ResponseError | null>(null);
  const questerClient = useMemo(() => new Client(), []);

  useEffect(() => {
    setLoading(true);
    questerClient.cluster().then(
      (cluster) => {
        setResponseError(null);
        setLoading(false);
        setCluster(cluster);
      },
      (error) => {
        setLoading(false);
        setResponseError(error);
      }
    );
  }, [questerClient]);

  const renderResult = () => {
    if (responseError !== null) {
      return ErrorResponseDisplay(responseError);
    }
    if (loading || cluster == null) {
      return <Loader />;
    }
    return <JsonEditor content={cluster} resizeOnMount={false} />
  }

  return (
    <ViewUnderAppBarBox>
      <FullBoxContainer>
        <QBreadcrumbs aria-label="breadcrumb">
          <Typography color="text.primary">Cluster</Typography>
        </QBreadcrumbs>
        <FullBoxContainer sx={{ px: 0 }}>
          { renderResult() }
        </FullBoxContainer>
      </FullBoxContainer>
      { ApiUrlFooter('api/v1/cluster') }
    </ViewUnderAppBarBox>
  );
}

export default ClusterView;

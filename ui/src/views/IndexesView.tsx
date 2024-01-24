

import { Box, Typography } from '@mui/material';
import { useEffect, useMemo, useState } from 'react';
import IndexesTable from '../components/IndexesTable';
import { Client } from '../services/client';
import Loader from '../components/Loader';
import { IndexMetadata, ResponseError } from '../utils/models';
import { ViewUnderAppBarBox, FullBoxContainer, QBreadcrumbs } from '../components/LayoutUtils';
import ApiUrlFooter from '../components/ApiUrlFooter';
import ErrorResponseDisplay from '../components/ResponseErrorDisplay';

function IndexesView() {
  const [loading, setLoading] = useState(false);
  const [responseError, setResponseError] = useState<ResponseError | null>(null);
  const [indexesMetadata, setIndexesMetadata] = useState<IndexMetadata[]>();
  const questerClient = useMemo(() => new Client(), []);

  const renderFetchIndexesResult = () => {
    if (responseError !== null) {
      return ErrorResponseDisplay(responseError);
    }
    if (loading || indexesMetadata === undefined) {
      return <Loader />;
    }
    if (indexesMetadata.length > 0) {
      return <FullBoxContainer sx={{ px: 0 }}>
          <IndexesTable indexesMetadata={indexesMetadata} />
        </FullBoxContainer>
    }
    return <Box>
        You have no index registered in your metastore.
      </Box>
  }

  useEffect(() => {
    setLoading(true);
    questerClient.listIndexes().then(
      (indexesMetadata) => {
        setResponseError(null);
        setLoading(false);
        setIndexesMetadata(indexesMetadata);
      },
      (error) => {
        setLoading(false);
        setResponseError(error);
      }
    );
  }, [questerClient]);

  return (
    <ViewUnderAppBarBox>
      <FullBoxContainer>
        <QBreadcrumbs aria-label="breadcrumb">
          <Typography color="text.primary">Indexes</Typography>
        </QBreadcrumbs>
        { renderFetchIndexesResult() }
      </FullBoxContainer>
      { ApiUrlFooter('api/v1/indexes') }
    </ViewUnderAppBarBox>
  );
}

export default IndexesView;

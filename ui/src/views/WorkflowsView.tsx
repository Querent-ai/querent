import {
  Box,
  Typography,
  Table,
  TableContainer,
  TableHead,
  TableBody,
  TableRow,
  TableCell,
  Paper,
  TablePagination,
  Checkbox,
  styled,
  CardContent,
  Card,
  Button,
} from '@mui/material';
import { useEffect, useMemo, useState } from 'react';
import { Client } from '../services/client';
import Loader from '../components/Loader';
import { ResponseError, PipelinesMetadata, IndexingStatistics } from '../utils/models';
import {
  ViewUnderAppBarBox,
  FullBoxContainer,
  QBreadcrumbs,
} from '../components/LayoutUtils';
import ApiUrlFooter from '../components/ApiUrlFooter';
import ErrorResponseDisplay from '../components/ResponseErrorDisplay';
import DeleteIcon from '@mui/icons-material/Delete';
import InfoIcon from '@mui/icons-material/Info';
import StatisticsView from '../components/SemanticStatisticsView';

// StyledTableCell for custom styling
const StyledTableCell = styled(TableCell)(({ theme }) => ({
  backgroundColor: theme.palette.primary.main,
  color: theme.palette.common.white,
}));

function WorkflowsView() {
  let healthyPipelineMap: Map<string, boolean> = new Map();
  const [showStatistics, setShowStatistics] = useState(false);
  const [loading, setLoading] = useState(false);
  const [responseError, setResponseError] = useState<ResponseError | null>(null);
  const [semanticServiceCounters, setSemanticServiceCounters] = useState({
    num_failed_pipelines: 0,
    num_running_pipelines: 0,
    num_successful_pipelines: 0,
  });
  const [pipelinesMetadata, setPipelinesMetadata] = useState<PipelinesMetadata | undefined>(undefined);
  const [statistics, setStatistics] = useState<IndexingStatistics | undefined>(undefined);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(5);
  const [selectedRows, setSelectedRows] = useState<string[]>([]);
  const questerClient = useMemo(() => new Client(), []);

  const handleSelectAllClick = (event: React.ChangeEvent<HTMLInputElement>) => {
    if (event.target.checked) {
      const allIds = pipelinesMetadata?.pipelines.map((pipeline) => pipeline.pipeline_id) || [];
      setSelectedRows(allIds);
    } else {
      setSelectedRows([]);
    }
  };

  const handleRowClick = (pipelineId: string) => {
    const selectedIndex = selectedRows.indexOf(pipelineId);
    let newSelected: string[] = [];

    if (selectedIndex === -1) {
      newSelected = newSelected.concat(selectedRows, pipelineId);
    } else if (selectedIndex === 0) {
      newSelected = newSelected.concat(selectedRows.slice(1));
    } else if (selectedIndex === selectedRows.length - 1) {
      newSelected = newSelected.concat(selectedRows.slice(0, -1));
    } else if (selectedIndex > 0) {
      newSelected = newSelected.concat(
        selectedRows.slice(0, selectedIndex),
        selectedRows.slice(selectedIndex + 1)
      );
    }

    setSelectedRows(newSelected);
  };

  const isSelected = (pipelineId: string) => selectedRows.indexOf(pipelineId) !== -1;

  const renderSemanticServiceCounters = () => {
    if (responseError !== null) {
      return ErrorResponseDisplay(responseError);
    }
    if (loading) {
      return <Loader />;
    }

    return (
      <Card>
        <CardContent>
          <Typography variant="h5" gutterBottom>
            Semantic Pipelines Overview
          </Typography>
          <Typography>
            Running Pipelines: {semanticServiceCounters.num_running_pipelines}
          </Typography>
          <Typography>
            Successful Pipelines: {semanticServiceCounters.num_successful_pipelines}
          </Typography>
          <Typography>
            Failed Pipelines: {semanticServiceCounters.num_failed_pipelines}
          </Typography>
        </CardContent>
      </Card>
    );
  };

  const renderPipelinesTable = () => {
    if (pipelinesMetadata === undefined) {
      return <Loader />;
    }

    const emptyRows =
      rowsPerPage -
      Math.min(
        rowsPerPage,
        pipelinesMetadata.pipelines.length - page * rowsPerPage
      );
    pipelinesMetadata.pipelines.forEach((pipeline) => {
      healthyPipelineMap.set(pipeline.pipeline_id, true);
    });
    
    return (
      <>
        <Box>
          <Button
            aria-label="Delete"
            variant="contained"
            color="error"
            onClick={() => handleDeleteAction(selectedRows)}
            disabled={selectedRows.length === 0}
            style={{ backgroundColor: selectedRows.length > 0 ? '#ff1744' : 'inherit' }}
          >
            <DeleteIcon />
          </Button>
          <></>
          <Button
            aria-label="Info"
            variant="contained"
            color="info"
            onClick={() => handleInfoAction(selectedRows)}
            disabled={selectedRows.length !== 1}
            style={{ backgroundColor: selectedRows.length === 1 ? '#1976D2' : 'inherit' }}
          >
            <InfoIcon />
          </Button>
        </Box>
        <TableContainer component={Paper}>
          <Table>
            <TableHead>
              <TableRow>
                <StyledTableCell>
                  <Checkbox
                    indeterminate={selectedRows.length > 0 && selectedRows.length < pipelinesMetadata.pipelines.length}
                    checked={selectedRows.length === pipelinesMetadata.pipelines.length}
                    onChange={handleSelectAllClick}
                  />
                </StyledTableCell>
                <StyledTableCell>Pipeline ID</StyledTableCell>
                <StyledTableCell>Name</StyledTableCell>
                <StyledTableCell>Status</StyledTableCell>
                {/* Add more table headers based on your PipelinesMetadata structure */}
              </TableRow>
            </TableHead>
            <TableBody>
              {(rowsPerPage > 0
                ? pipelinesMetadata.pipelines.slice(
                    page * rowsPerPage,
                    page * rowsPerPage + rowsPerPage
                  )
                : pipelinesMetadata.pipelines
              ).map((pipeline) => (
                <TableRow
                  key={pipeline.pipeline_id}
                  selected={isSelected(pipeline.pipeline_id)}
                  onClick={() => handleRowClick(pipeline.pipeline_id)}
                >
                  <TableCell>
                    <Checkbox
                      checked={isSelected(pipeline.pipeline_id)}
                    />
                  </TableCell>
                  <TableCell>{pipeline.pipeline_id}</TableCell>
                  <TableCell>{pipeline.name}</TableCell>
                  <TableCell>
                    {healthyPipelineMap.get(pipeline.pipeline_id) === true ? '✅' : '❌'}
                  </TableCell>
                  {/* Add more table cells based on your PipelinesMetadata structure */}
                </TableRow>
              ))}
              {emptyRows > 0 && (
                <TableRow style={{ height: 53 * emptyRows }}>
                  <TableCell colSpan={4} />
                </TableRow>
              )}
            </TableBody>
          </Table>
          <TablePagination
            rowsPerPageOptions={[5, 10, 25]}
            component="div"
            count={pipelinesMetadata.pipelines.length}
            rowsPerPage={rowsPerPage}
            page={page}
            onPageChange={(_event, newPage) => setPage(newPage)}
            onRowsPerPageChange={(event) => {
              setRowsPerPage(parseInt(event.target.value, 10));
              setPage(0);
            }}
          />
        </TableContainer>
      </>
    );
  };

  const handleDeleteAction = (selectedRows: string[]) => {  
    // Assuming you have an endpoint for deleting pipelines
    selectedRows.forEach((pipelineId) => {
      questerClient.deleteSemanticPipeline(pipelineId)
        .then(() => {
          console.log(`Pipeline with ID ${pipelineId} deleted successfully.`);
          // Fetch the updated data after deletion
          fetchPipelinesData();
        })
        .catch((error) => {
          console.error(`Error deleting pipeline with ID ${pipelineId}:`, error);
          // Handle the error as needed
        });
    });
  };

  const fetchPipelinesData = () => {
    setLoading(true);

    // Fetch Semantic Service Counters
    const fetchSemanticServiceCounters = questerClient.getSemanticServiceCounters();

    // Fetch Pipelines Metadata
    const fetchPipelinesMetadata = questerClient.getSemanticPipelinesMetadata();

    // Execute both requests in parallel
    Promise.all([fetchSemanticServiceCounters, fetchPipelinesMetadata])
      .then(([counters, metadata]) => {
        setResponseError(null);
        setLoading(false);
        setSemanticServiceCounters(counters);
        setPipelinesMetadata(metadata);
      })
      .catch((error) => {
        setLoading(false);
        setResponseError(error);
      });
  };

  const handleInfoAction = (selectedRows: string[]) => {
    // Implement your logic for info action
  
    // Assuming you have an endpoint for getting pipeline info
    const pipelineId = selectedRows[0];
    if (pipelineId === undefined) {
      return;
    }
    questerClient.getPipelineDescription(pipelineId)
      .then((_info) => {
        healthyPipelineMap.set(pipelineId, true);
        setShowStatistics(true); // Set state to show statistics
        setStatistics(_info); // Set statistics
        // Optionally, you can display the info or perform any other actions
      })
      .catch((error) => {
        healthyPipelineMap.set(pipelineId, false);
        console.error(`Error getting info for pipeline with ID ${pipelineId}:`, error);
        // Handle the error as needed
      });
  };

  const renderInfoSection = () => {
    if (showStatistics && statistics !== undefined) {
      return <StatisticsView statistics={statistics} />;
    }
    return null;
  };

  useEffect(() => {
    setLoading(true);

    // Fetch Semantic Service Counters
    const fetchSemanticServiceCounters = questerClient.getSemanticServiceCounters();

    // Fetch Pipelines Metadata
    const fetchPipelinesMetadata = questerClient.getSemanticPipelinesMetadata();

    // Execute both requests in parallel
    Promise.all([fetchSemanticServiceCounters, fetchPipelinesMetadata])
      .then(([counters, metadata]) => {
        setResponseError(null);
        setLoading(false);
        setSemanticServiceCounters(counters);
        setPipelinesMetadata(metadata);
      })
      .catch((error) => {
        setLoading(false);
        setResponseError(error);
      });
  }, [questerClient]);

  return (
    <ViewUnderAppBarBox>
      <FullBoxContainer>
        <QBreadcrumbs aria-label="breadcrumb">
          <Typography color="text.primary">Semantic Pipelines</Typography>
        </QBreadcrumbs>
        {renderSemanticServiceCounters()}
        {renderPipelinesTable()}
        {renderInfoSection()}
      </FullBoxContainer>
      {ApiUrlFooter('api/v1/semantics')}
    </ViewUnderAppBarBox>
  );
}

export default WorkflowsView;

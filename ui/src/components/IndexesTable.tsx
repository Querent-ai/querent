

import { Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow } from "@mui/material";
import dayjs from 'dayjs';
import utc from "dayjs/plugin/utc"
import { IndexMetadata } from "../utils/models";
import { useNavigate } from "react-router-dom";
dayjs.extend(utc);


const IndexesTable = ({ indexesMetadata }: Readonly<{indexesMetadata: IndexMetadata[]}>) => {
  const navigate = useNavigate();
  const handleClick = function(indexId: string) {
    navigate(`/indexes/${indexId}`);
  }

  return (
    <TableContainer component={Paper}>
      <Table sx={{ minWidth: 650 }} aria-label="Indexes">
        <TableHead>
          <TableRow>
            <TableCell align="left">ID</TableCell>
            <TableCell align="left">URI</TableCell>
            <TableCell align="left">Created on</TableCell>
            <TableCell align="left">Sources</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {indexesMetadata.map((indexMetadata) => (
            <TableRow
              key={indexMetadata.index_config.index_id}
              sx={{ '&:last-child td, &:last-child th': { border: 0 }, cursor: "pointer"}}
              hover={true}
              onClick={() => handleClick(indexMetadata.index_config.index_id)}
            >
              <TableCell component="th" scope="row">
                {indexMetadata.index_config.index_id}
              </TableCell>
              <TableCell align="left">{indexMetadata.index_config.index_uri}</TableCell>
              <TableCell align="left">{ dayjs.unix(indexMetadata.create_timestamp).utc().format("YYYY/MM/DD HH:mm") }</TableCell>
              <TableCell align="left">{ indexMetadata.sources?.length || 'None'}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );
};

export default IndexesTable;

use std::{sync::Arc, time::Duration};

use common::{storage_config::MilvusConfig, SemanticKnowledgePayload, VectorPayload};
use milvus::{
	client::Client as MilvusClient,
	data::FieldColumn,
	schema::{CollectionSchemaBuilder, FieldSchema},
	value::ValueVec,
};

use crate::{storage::Storage, StorageError, StorageErrorKind, StorageResult};
use async_trait::async_trait;

pub struct MilvusStorage {
	pub client: Arc<MilvusClient>,
	pub config: MilvusConfig,
}

impl MilvusStorage {
	pub async fn new(config: MilvusConfig) -> StorageResult<Self> {
		let client_res = MilvusClient::with_timeout(
			config.url.clone(),
			Duration::from_secs(5),
			Some(config.username.clone()),
			Some(config.password.clone()),
		)
		.await;
		match client_res {
			Ok(client) => Ok(MilvusStorage { client: Arc::new(client), config }),
			Err(err) => {
				log::error!("Milvus client creation failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Internal,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}
}

#[async_trait]
impl Storage for MilvusStorage {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let _ = self.client.list_collections().await?;
		Ok(())
	}

	async fn insert_vector(
		&self,
		_collection_id: String,
		_payload: &Vec<(String, VectorPayload)>,
	) -> StorageResult<()> {
		let new_collection_id = format!("_{}", _collection_id);
		for (id, payload) in _payload {
			let result =
				self.insert_or_create_collection(new_collection_id.as_str(), id, payload).await;
			if let Err(err) = result {
				log::error!("Vector insertion failed: {:?}", err);
				return Err(err);
			}
		}
		Ok(())
	}

	async fn insert_graph(
		&self,
		_payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your insert_graph implementation here
		Ok(())
	}

	async fn index_knowledge(
		&self,
		_payload: &Vec<(String, SemanticKnowledgePayload)>,
	) -> StorageResult<()> {
		// Your index_triples implementation here
		Ok(())
	}
}

impl MilvusStorage {
	async fn insert_or_create_collection(
		&self,
		collection_name: &str, // this is workflow id
		id: &str,              // this is document id
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let collection = self.client.get_collection(collection_name).await;
		match collection {
			Ok(collection) => {
				log::debug!("Collection found: {:?}", collection);
				self.insert_into_collection(&collection, id, payload).await
			},
			Err(_err) => {
				log::error!("Error in milvus client: {:?}", _err);
				self.create_and_insert_collection(collection_name, id, payload).await
			},
		}
	}

	async fn insert_into_collection(
		&self,
		collection: &milvus::collection::Collection,
		id: &str,
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let has_partition = collection.has_partition(&payload.namespace).await;
		match has_partition {
			Ok(has_partition) =>
				if !has_partition {
					let partition = collection.create_partition(&payload.namespace).await;
					match partition {
						Ok(partition) => {
							log::debug!("Partition created: {:?}", partition);
						},
						Err(err) => {
							log::error!("Partition creation failed: {:?}", err);
							return Err(StorageError {
								kind: StorageErrorKind::PartitionCreation,
								source: Arc::new(anyhow::Error::from(err)),
							});
						},
					}
				},
			Err(err) => {
				log::error!("Partition check failed: {:?}", err);
				return Err(StorageError {
					kind: StorageErrorKind::PartitionCreation,
					source: Arc::new(anyhow::Error::from(err)),
				});
			},
		}

		let knowledge_field = FieldColumn::new(
			collection.schema().get_field("knowledge").unwrap(),
			ValueVec::String(vec![payload.id.clone()]),
		);
		let relationship_field = FieldColumn::new(
			collection.schema().get_field("relationship").unwrap(),
			ValueVec::String(vec![payload.namespace.clone()]),
		);
		let document_field = FieldColumn::new(
			collection.schema().get_field("document").unwrap(),
			ValueVec::String(vec![id.to_string()]),
		);
		let embeddings_field = FieldColumn::new(
			collection.schema().get_field("embeddings").unwrap(),
			payload.embeddings.clone(),
		);

		let records = vec![knowledge_field, relationship_field, document_field, embeddings_field];
		let insert_result = collection.insert(records, Some(payload.namespace.as_str())).await;
		match insert_result {
			Ok(insert_result) => {
				log::debug!("Insert result: {:?}", insert_result);
				let flushed_collection = collection.flush().await;
				match flushed_collection {
					Ok(flushed_collection) => {
						log::debug!("Collection flushed: {:?}", flushed_collection);
						Ok(())
					},
					Err(err) => {
						log::error!("Flush failed: {:?}", err);
						Err(StorageError {
							kind: StorageErrorKind::Insertion,
							source: Arc::new(anyhow::Error::from(err)),
						})
					},
				}
			},
			Err(err) => {
				log::error!("Insert failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::Insertion,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}

	async fn create_and_insert_collection(
		&self,
		collection_name: &str,
		id: &str,
		payload: &VectorPayload,
	) -> StorageResult<()> {
		let description = format!("Semantic collection adhering to s->p->o ={:?}", payload.id);
		let new_coll = CollectionSchemaBuilder::new(collection_name, description.as_str())
			.add_field(FieldSchema::new_primary_int64("id", "auto id for each vector", true))
			.add_field(FieldSchema::new_varchar("knowledge", "subject, predicate, object", 500))
			.add_field(FieldSchema::new_varchar(
				"relationship",
				"predicate associated with embedding",
				500,
			))
			.add_field(FieldSchema::new_varchar(
				"document",
				"document associated with embedding",
				500,
			))
			.add_field(FieldSchema::new_float_vector(
				"embeddings",
				"semantic vector embeddings",
				payload.size as i64,
			))
			.build();

		match new_coll {
			Ok(new_coll) => {
				let collection = self.client.create_collection(new_coll, None).await;
				match collection {
					Ok(collection) => {
						log::debug!("Collection created: {:?}", collection);
						self.insert_into_collection(&collection, id, payload).await
					},
					Err(err) => {
						log::error!("Collection creation failed: {:?}", err);
						Err(StorageError {
							kind: StorageErrorKind::CollectionCreation,
							source: Arc::new(anyhow::Error::from(err)),
						})
					},
				}
			},
			Err(err) => {
				log::error!("Collection builder failed: {:?}", err);
				Err(StorageError {
					kind: StorageErrorKind::CollectionBuilding,
					source: Arc::new(anyhow::Error::from(err)),
				})
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_insert_vector() {
        // Assume MilvusStorage::new is correctly implemented to handle connection setup.
        let storage_config = MilvusConfig {
            url: "http://localhost:19530".to_string(),
            username: "".to_string(),
            password: "".to_string(),
        };
        let storage = MilvusStorage::new(storage_config).await.unwrap();

        // Provided payload adjusted for Rust
        let embeddings: Vec<f32> = vec![
            0.0168774351477623, -0.02473863773047924, 0.08342848718166351, 0.05220113694667816, -0.051285888999700546, -0.02160704880952835, -0.14544075727462769, 0.009335435926914215, -0.03842026740312576, -0.06677619367837906, -0.11946547031402588, 0.038361240178346634, -0.04201112687587738, -0.03941815346479416, -0.00818631425499916, -0.0025189814623445272, 0.0805322676897049, -0.04540707543492317, 0.06900567561388016, -0.0539812333881855, -0.04545339569449425, 0.05319667607545853, 0.05052277818322182, 0.03182961046695709, -0.023138904944062233, 0.04489652067422867, -0.05840165540575981, 0.04498237743973732, -0.0074174306355416775, -0.011436556465923786, 0.0047922320663928986, 0.08194663375616074, -0.033605027943849564, -0.06518253684043884, -0.016110964119434357, 0.02163287065923214, -0.05563156306743622, -0.011725652031600475, 0.0278729610145092, 0.009782524779438972, -0.03433762118220329, -0.09937843680381775, 0.0642143040895462, -0.02156674489378929, -0.07031495124101639, -0.05042269825935364, -0.03805641457438469, 0.002043040469288826, 0.036709073930978775, 0.021840842440724373, 0.05121251195669174, 0.008003591559827328, -0.04620864614844322, -0.013205030001699924, 0.07930979877710342, 0.027889160439372063, -0.057479240000247955, -0.08615680783987045, -0.01702786423265934, 0.024664340540766716, 0.042513176798820496, 0.07131560891866684, 0.005905502010136843, 0.016944216564297676, -0.003519928315654397, 0.010387605987489223, -0.04940903186798096, -0.08692266792058945, 0.01989026553928852, -0.08311661332845688, 0.03262312710285187, -0.003558138618245721, -0.08150030672550201, -0.04203197360038757, 0.003723866306245327, 0.05705210939049721, 0.07357092946767807, 0.01364993304014206, 0.10031905770301819, -0.08606190979480743, -0.02276902087032795, -0.028837082907557487, 0.05388037860393524, -0.1452949345111847, -0.07542111724615097, -0.043040286749601364, 0.09104114770889282, 0.0161473136395216, 0.024531356990337372, -0.003988891374319792, 0.05356590822339058, -0.004400306381285191, -0.10311596095561981, -0.009832543320953846, 0.10946295410394669, 0.027561360970139503, -0.00821763090789318, -0.02697383239865303, -0.05381276458501816, 0.012951094657182693, 0.07057410478591919, 0.0472826212644577, -0.06566429883241653, -0.09585657715797424, 0.009087931364774704, 0.039309266954660416, 0.001549118896946311, 0.03840886428952217, -0.015845490619540215, 0.025711409747600555, 0.07673590630292892, -0.049066126346588135, 0.02177552506327629, -0.007265402004122734, -0.01946171559393406, -0.08215844631195068, -0.07054311037063599, -0.03411214426159859, -0.01694772019982338, -0.00646223546937108, -0.042551856487989426, -0.03164905309677124, -0.015045501291751862, 0.022718679159879684, 0.03680476173758507, 0.025209182873368263, -0.05344066396355629, 4.165403905073116e-33, 0.009866856969892979, -0.07635921984910965, 0.013200042769312859, -0.016313999891281128, -0.020286565646529198, 0.004511426202952862, -0.013412082567811012, 0.03348471596837044, -0.07325669378042221, 0.040163327008485794, 0.021139487624168396, 0.02853340283036232, -0.022274373099207878, 0.014703873544931412, -0.05757609009742737, -0.09910541772842407, -0.03758542612195015, 0.0024536310229450464, 0.01415186282247305, -0.06784151494503021, -0.07247854024171829, 0.004043382126837969, -0.026491910219192505, -0.018503401428461075, 0.03022831492125988, 0.005283795762807131, -0.04814857617020607, -0.011469747871160507, -0.0726868212223053, 0.013230888172984123, -0.0019286926835775375, -0.046928975731134415, -0.012781953439116478, -0.07733823359012604, 0.06652522832155228, -0.0005580178112722933, 0.08396165817975998, 0.006181728560477495, -0.13494744896888733, 0.01627029851078987, -0.04447316750884056, -0.006843085400760174, -0.0074546560645103455, -0.006339933257550001, -0.01531272940337658, 0.005881189368665218, 0.00654362328350544, -0.018833983689546585, -0.03397207707166672, -0.03602798283100128, -0.04912665858864784, 0.003345032222568989, -0.02943441830575466, 0.01495272945612669, -0.011441953480243683, 0.05510178580880165, 0.035366401076316833, -0.06910791993141174, -0.003583775367587805, 0.032761797308921814, -0.06629043072462082, 0.10589878261089325, -0.04693916067481041, -0.012611470185220242, -0.03920392319560051, -0.002484732773154974, 0.04620010033249855, 0.031355585902929306, -0.026794206351041794, 0.10601083189249039, 0.1327933371067047, -0.018946729600429535, 0.019274603575468063, -0.06746817380189896, -0.022397790104150772, 0.02271655760705471, 0.06090734526515007, 0.05066154524683952, 0.031031591817736626, 0.06402446329593658, -0.05334138497710228, -0.04979322850704193, 0.01657039299607277, -0.022908175364136696, 0.03521406278014183, 0.04832370951771736, -0.050894469022750854, -0.019307078793644905, 0.06314395368099213, 0.012032714672386646, -0.05014624446630478, 0.07104429602622986, -0.014850392006337643, -0.024831503629684448, 0.06363866478204727, -5.930363489768671e-33, -0.0010440689511597157, 0.022551901638507843, -0.010656171478331089, 0.0741954892873764, 0.0026306186337023973, -0.03373681753873825, 0.09984590858221054, -0.02020726539194584, 0.06387317180633545, -0.027789853513240814, -0.08218865096569061, 0.08724474906921387, 0.017137974500656128, -0.06714123487472534, 0.01709212362766266, -0.09141712635755539, -0.03356151655316353, -0.030554883182048798, -0.005140150431543589, 0.05965881794691086, -0.06430681049823761, 0.03124699369072914, -0.04497101530432701, 0.037032101303339005, -0.022470850497484207, 0.017089026048779488, -0.07372583448886871, -0.03914332389831543, -0.10819695889949799, -0.05217158421874046, 0.003635512897744775, 0.013878975994884968, 0.012259458191692829, 0.011529997922480106, -0.06647251546382904, 0.014441072940826416, 0.0772298127412796, 0.009192406199872494, -0.047257814556360245, 0.0009663437958806753, 0.035622190684080124, 0.09093190729618073, -0.0032224494498223066, 0.046146515756845474, 0.008867665193974972, 0.00945991650223732, 0.0930013656616211, -0.03813811019062996, 0.026157721877098083, 0.06985271722078323, -0.020468275994062424, -0.10676197707653046, -0.027608634904026985, 0.08457552641630173, -0.049662500619888306, 0.041233040392398834, -0.02605554461479187, 0.05362899228930473, -0.029106495901942253, 0.07035847008228302, 0.038809195160865784, 0.11287698149681091, -0.014675542712211609, 0.060562215745449066, 0.0762038379907608, 0.01565535180270672, -0.09715051203966141, -0.04723691940307617, 0.030417675152420998, 0.01943036913871765, -0.03966127708554268, -0.04654771462082863, -0.00514251459389925, 0.015919242054224014, 0.05823274329304695, 0.054392263293266296, 0.03853018581867218, 0.01011577621102333, 0.014123624190688133, 0.08380627632141113, 0.0244181789457798, 0.0038165159057825804, -0.06477531790733337, 0.02732235938310623, 0.03327915444970131, 0.007346701342612505, 0.0028210363816469908, -0.06395537406206131, 0.03424294665455818, 0.05245060846209526, -0.055291566997766495, 0.051441773772239685, -0.11584176123142242, -0.07742477208375931, -0.0320034883916378, -4.1641083470267404e-08, -0.048972319811582565, 0.10141859203577042, -0.015451534651219845, 0.039658717811107635, 0.0504334419965744, -0.014123652130365372, 0.09565389901399612, 0.10158748179674149, 0.002334865042939782, 0.0029223719611763954, 0.007382504642009735, -0.04364606365561485, -0.02273465320467949, -0.025698818266391754, 0.019899671897292137, -0.04264162480831146, -0.025589989498257637, 0.0741720050573349, -0.042433347553014755, -0.0721844732761383, 0.027941299602389336, 0.03644467145204544, 0.10426587611436844, 0.004252371843904257, -0.06348146498203278, -0.012325935997068882, 0.0584472231566906, 0.04665137827396393, -0.03713950514793396, 0.03511423617601395, 0.0063893748447299, 0.011651864275336266, -0.04180409014225006, -0.01871485263109207, 0.14710888266563416, -0.005030917003750801, -0.002655130112543702, 0.08570198714733124, 0.037130001932382584, 0.07120367139577866, -0.09475141763687134, 0.03749271109700203, -0.016684724017977715, -0.044582296162843704, -0.03873072937130928, 0.03555674850940704, -0.10662214457988739, 0.08450291305780411, 0.011333310045301914, -0.01882963441312313, 0.042215511202812195, 0.014848827384412289, -0.003502971027046442, -0.00842451024800539, 0.11267822235822678, 0.015071590431034565, 0.05076431855559349, 0.014697236940264702, -0.08384405076503754, -0.05143100395798683, -0.0033587669022381306, -0.02306164987385273, 0.03600551187992096, -0.060147613286972046
        ];
        let payload = VectorPayload {
            id: "gas_well_exceeding_rates_17_mmcf/d".to_string(),
            namespace: "exceeding_rates".to_string(),
            size: 384,
            embeddings,
        };

        // Adjust the collection name to the one you're using
        let collection_name = "testing_database_2".to_string();

        // Attempt to insert the payload
        let result = storage.insert_vector(
            collection_name,
            &vec![("document_id_example".to_string(), payload)] // Example document ID
        ).await;

        // Check if the insertion was successful
        assert!(result.is_ok(), "Insertion failed: {:?}", result.err());
    }
}
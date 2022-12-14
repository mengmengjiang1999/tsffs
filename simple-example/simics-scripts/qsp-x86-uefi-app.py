from sim_params import params
import simics
import commands
import io, contextlib

args = [[name, commands.param_val_to_str(value)]
        for (name, value) in params.items()]
simics.SIM_run_command_file_params(
    simics.SIM_lookup_file("%simics%/targets/qsp-x86-fuzzing/run-uefi-app.simics"),
    True, args)
#params.setdefault("system", simenv.system)
#params.setdefault("eth_link", simenv.eth_link)

#conf.board.gfx.dev.console = None
if SIM_get_batch_mode():
  SIM_log_info(1, conf.sim, 0, 'Batch mode detected. Disconnecting console from VGA')
  conf.board.mb.gpu.vga.console=None

#Reach start state of test (indicated by MAGIC(42) in on-target test harness
SIM_run_command('bp.hap.run-until name = Core_Magic_Instruction index = 42')

#Create our glue objects
SIM_create_object('confuse_ll','fuzz_if',[])
SIM_create_object('confuse_dio','dio_if',[])
conf.dio_if.pipe = conf.magic_pipe

bp_id=SIM_run_command('b 0x00000000def6249c') #taken from IDT (UD handler)
conf.dio_if.iface.confuse_dio.add_abnormal_exit(bp_id, 'Application crash (UD)')

#Enable in memory snapshot feature
SIM_run_command('enable-unsupported-feature internals')
#SIM_run_command('enable-unsupported-feature selfprof')

#Now read some session data we get from the interface
#TODO: make file name part of YML 
#TODO: use some standard format like json
#TODO: parse out shared mem file for data I/O
#TODO: parse out shared mem file for AFL area
if_pid=0
with open('./_if_data_.tmp') as if_cfg:
  for line in if_cfg:
      if line.strip().startswith('if_pid'):
          if_pid=int(line.strip().split(':')[1])

SIM_run_command('save-snapshot name = origin')

#Check that we have our snapshot as index 0 (which is currently hard coded in the restore code
cmd_output = io.StringIO()
with contextlib.redirect_stdout(cmd_output):
    SIM_run_command('list-snapshots')
res = cmd_output.getvalue()
ckpt_id = -1
for line in res.split('\n'):
    line = line.split()
    if len(line) > 2 and line[1]=='origin':
        ckpt_id = int(line[0])
if ckpt_id != 0:
    SIM_log_error(conf.fuzz_if, 0, 'Microcheckpoint ID %d. Must be zero!'%(ckpt_id))
else:
    SIM_log_info(1, conf.fuzz_if, 0, 'Microcheckpoint ID %d'%(ckpt_id))

#SIM_run_command('start-selfprof') #or use vtune at that point

#arm auto sender of SIGUSR2 whenever the sim stops. Since right now sim is stopped
# this has no immediate effect
#conf.fuzz_if.arm_auto_send_usr2 = SIM_run_command('pid') #if_pid
conf.fuzz_if.arm_auto_send_usr2 = if_pid

#Tell interface that we have reached the start state and the snapshot is ready
#conf.fuzz_if.send_usr2 = SIM_run_command('pid') #if_pid
conf.fuzz_if.send_usr2 = if_pid
conf.dio_if.if_pid = if_pid





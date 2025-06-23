import json
import uuid
from ortools.sat.python import cp_model

class DataManager:
    """A helper class to load, process, and manage the data."""
    def __init__(self, data):
        self.data = data
        self._create_mappings()
        self._preprocess_data()

    def _create_mappings(self):
        """Create ID-to-index and index-to-ID mappings for easier lookup."""
        self.time = {item['id']: item for item in self.data['time']}
        self.day = {item['id']: item for item in self.data['day']}
        self.courses = {item['id']: item for item in self.data['courses']}
        self.teachers = {item['id']: item for item in self.data['teachers']}
        
        self.time_list = self.data['time']
        self.day_list = self.data['day']
        self.course_list = self.data['courses']
        self.teacher_list = self.data['teachers']
        self.campus_list = self.data['campuses']
        
        self.venue_list = []
        for campus in self.campus_list:
            for venue in campus['venues']:
                self.venue_list.append({**venue, 'campus_id': campus['id']})

        self.time_id_to_idx = {t['id']: i for i, t in enumerate(self.time_list)}
        self.day_id_to_idx = {d['id']: i for i, d in enumerate(self.day_list)}
        self.course_id_to_idx = {c['id']: i for i, c in enumerate(self.course_list)}
        self.teacher_id_to_idx = {t['id']: i for i, t in enumerate(self.teacher_list)}
        self.venue_id_to_idx = {v['id']: i for i, v in enumerate(self.venue_list)}
        self.campus_id_to_idx = {c['id']: i for i, c in enumerate(self.campus_list)}
        
        self.venue_idx_to_campus_idx = {
            self.venue_id_to_idx[v['id']]: self.campus_id_to_idx[c['id']]
            for c in self.campus_list for v in c['venues']
        }
        
        self.density_map = {}
        for c_idx, campus in enumerate(self.campus_list):
            for density in campus.get('schedule_density', []):
                d_idx = self.day_id_to_idx[density['day_id']]
                t_idx = self.time_id_to_idx[density['time_id']]
                self.density_map[(c_idx, d_idx, t_idx)] = density['count']


    def _preprocess_data(self):
        """Pre-calculate values from existing schedules."""
        self.existing_teacher_hours = {t_idx: 0 for t_idx in range(len(self.teacher_list))}
        self.existing_venue_usage = {}
        self.existing_teacher_day_campus = {}
        self.teacher_has_existing_campus = {
            t_idx: {c_idx: False for c_idx in range(len(self.campus_list))}
            for t_idx in range(len(self.teacher_list))
        }
        self.existing_teacher_day_time_busy = {}
        
        self.existing_work_days = {t_idx: set() for t_idx in range(len(self.teacher_list))}

        for t_idx, teacher in enumerate(self.teacher_list):
            for scheduled in teacher.get('scheduled', []):
                time_id = scheduled['time_id']
                day_id = scheduled['day_id']
                venue_id = scheduled['venue_id']
                campus_id = scheduled['campus_id']
                
                hours = self.time[time_id]['corresponding_hours']
                self.existing_teacher_hours[t_idx] += hours
                
                d_idx = self.day_id_to_idx[day_id]
                i_idx = self.time_id_to_idx[time_id]
                v_idx = self.venue_id_to_idx[venue_id]
                c_idx = self.campus_id_to_idx[campus_id]
                
                self.existing_work_days[t_idx].add(d_idx)

                key = (v_idx, d_idx, i_idx)
                self.existing_venue_usage[key] = self.existing_venue_usage.get(key, 0) + 1
                
                if (t_idx, d_idx) in self.existing_teacher_day_campus and self.existing_teacher_day_campus[(t_idx, d_idx)] != c_idx:
                    print(f"ERROR: Teacher {teacher['name']} has conflicting existing schedules on the same day across different campuses.")
                self.existing_teacher_day_campus[(t_idx, d_idx)] = c_idx
                self.teacher_has_existing_campus[t_idx][c_idx] = True
                
                self.existing_teacher_day_time_busy[(t_idx, d_idx, i_idx)] = True

def solve_scheduling(data_manager):
    """Builds and solves the CP-SAT model."""
    dm = data_manager
    model = cp_model.CpModel()

    x = {}
    
    for t_idx, teacher in enumerate(dm.teacher_list):
        for course_id in teacher['teaches']:
            c_idx = dm.course_id_to_idx[course_id]
            course = dm.course_list[c_idx]
            for place in course['place']:
                v_idx = dm.venue_id_to_idx[place['venue_id']]
                for d_idx in range(len(dm.day_list)):
                    for i_idx in range(len(dm.time_list)):
                        key = (t_idx, c_idx, d_idx, i_idx, v_idx)
                        x[key] = model.NewBoolVar(f'x_{t_idx}_{c_idx}_{d_idx}_{i_idx}_{v_idx}')

    print(f"Created {len(x)} potential assignment variables after pruning.")

    for t_idx, teacher in enumerate(dm.teacher_list):
        teacher_id = teacher['id']
        unavailable_slots = {(dm.day_id_to_idx[u['day_id']], dm.time_id_to_idx[u['time_id']]) for u in teacher.get('unavailable', [])}
        for d_idx in range(len(dm.day_list)):
            for i_idx in range(len(dm.time_list)):
                
                possible_classes_in_slot = [
                    var for key, var in x.items()
                    if key[0] == t_idx and key[2] == d_idx and key[3] == i_idx
                ]
                
                if dm.existing_teacher_day_time_busy.get((t_idx, d_idx, i_idx), False):
                    model.Add(sum(possible_classes_in_slot) == 0)
                else:
                    model.Add(sum(possible_classes_in_slot) <= 1)

                if (d_idx, i_idx) in unavailable_slots:
                    model.Add(sum(possible_classes_in_slot) == 0)
    
    for v_idx in range(len(dm.venue_list)):
        venue_capacity = dm.venue_list[v_idx]['capacity']
        for d_idx in range(len(dm.day_list)):
            for i_idx in range(len(dm.time_list)):
                existing_usage = dm.existing_venue_usage.get((v_idx, d_idx, i_idx), 0)
                new_usage = sum(
                    var for key, var in x.items()
                    if key[4] == v_idx and key[2] == d_idx and key[3] == i_idx
                )
                model.Add(new_usage + existing_usage <= venue_capacity)

    for t_idx, teacher in enumerate(dm.teacher_list):
        existing_hours = dm.existing_teacher_hours[t_idx]
        new_hours = sum(
            var * dm.time_list[key[3]]['corresponding_hours'] 
            for key, var in x.items() if key[0] == t_idx
        )
        model.Add(new_hours + existing_hours <= teacher['max_teaching_hours'])

    for t_idx, teacher in enumerate(dm.teacher_list):
        if teacher['is_only_shahe']:
            shahe_campus_idx = dm.campus_id_to_idx['138697dc-1591-4c16-b60e-d0057964be56']
            for key, var in x.items():
                if key[0] == t_idx:
                    venue_campus_idx = dm.venue_idx_to_campus_idx[key[4]]
                    model.AddImplication(var, model.NewConstant(venue_campus_idx == shahe_campus_idx))

    for t_idx in range(len(dm.teacher_list)):
        for d_idx in range(len(dm.day_list)):
            teacher_day_campus_vars = [model.NewBoolVar(f'teacher_{t_idx}_day_{d_idx}_campus_{c_idx}') for c_idx in range(len(dm.campus_list))]
            model.Add(sum(teacher_day_campus_vars) <= 1)
            
            if (t_idx, d_idx) in dm.existing_teacher_day_campus:
                fixed_campus_idx = dm.existing_teacher_day_campus[(t_idx, d_idx)]
                for c_idx, campus_var in enumerate(teacher_day_campus_vars):
                    model.Add(campus_var == (c_idx == fixed_campus_idx))

            for key, var in x.items():
                if key[0] == t_idx and key[2] == d_idx:
                    venue_campus_idx = dm.venue_idx_to_campus_idx[key[4]]
                    model.AddImplication(var, teacher_day_campus_vars[venue_campus_idx])

    for t_idx, teacher in enumerate(dm.teacher_list):
        if not teacher['is_only_shahe']:
            teacher_teaches_at_campus = [model.NewBoolVar(f'teacher_{t_idx}_teaches_at_campus_{c_idx}') for c_idx in range(len(dm.campus_list))]
            
            for c_idx in range(len(dm.campus_list)):
                has_existing = dm.teacher_has_existing_campus[t_idx][c_idx]
                new_classes_at_campus = [
                    var for key, var in x.items()
                    if key[0] == t_idx and dm.venue_idx_to_campus_idx[key[4]] == c_idx
                ]
                model.AddBoolOr([model.NewConstant(has_existing)] + new_classes_at_campus).OnlyEnforceIf(teacher_teaches_at_campus[c_idx])
                
            model.Add(sum(teacher_teaches_at_campus) == len(dm.campus_list))

    for t_idx in range(len(dm.teacher_list)):
        for d_idx in range(len(dm.day_list)):
            is_busy_in_slot = []
            for i_idx in range(len(dm.time_list)):
                slot_is_busy = model.NewBoolVar(f'teacher_{t_idx}_day_{d_idx}_time_{i_idx}_busy')
                has_existing = dm.existing_teacher_day_time_busy.get((t_idx, d_idx, i_idx), False)
                new_classes_in_slot = [
                    var for key, var in x.items()
                    if key[0] == t_idx and key[2] == d_idx and key[3] == i_idx
                ]
                
                model.AddBoolOr([model.NewConstant(has_existing)] + new_classes_in_slot).OnlyEnforceIf(slot_is_busy)
                model.Add(sum(new_classes_in_slot) == 0).OnlyEnforceIf(slot_is_busy.Not())
                if not has_existing:
                     model.Add(sum(new_classes_in_slot) > 0).OnlyEnforceIf(slot_is_busy)
                
                is_busy_in_slot.append(slot_is_busy)
            
            if len(is_busy_in_slot) == 3:
                model.AddBoolOr([is_busy_in_slot[0].Not(), is_busy_in_slot[1], is_busy_in_slot[2].Not()])

    for t_idx, teacher in enumerate(dm.teacher_list):
        num_existing_days = len(dm.existing_work_days[t_idx])

        if num_existing_days >= 4:
            for d_idx in range(len(dm.day_list)):
                if d_idx not in dm.existing_work_days[t_idx]:
                    new_classes_on_this_day = [
                        var for key, var in x.items()
                        if key[0] == t_idx and key[2] == d_idx
                    ]
                    if new_classes_on_this_day:
                        model.Add(sum(new_classes_on_this_day) == 0)
            continue 

        is_working_day_vars = []
        for d_idx in range(len(dm.day_list)):
            works_on_day = model.NewBoolVar(f'teacher_{t_idx}_works_on_day_{d_idx}')
            is_working_day_vars.append(works_on_day)

            new_classes_on_day = [
                var for key, var in x.items()
                if key[0] == t_idx and key[2] == d_idx
            ]
            has_existing_on_day = 1 if d_idx in dm.existing_work_days[t_idx] else 0

            total_activity_on_day = sum(new_classes_on_day) + has_existing_on_day

            model.Add(total_activity_on_day > 0).OnlyEnforceIf(works_on_day)
            
            model.Add(total_activity_on_day == 0).OnlyEnforceIf(works_on_day.Not())

        model.Add(sum(is_working_day_vars) <= 3)

    diff_vars = []
    for c_idx in range(len(dm.campus_list)):
        for d_idx in range(len(dm.day_list)):
            for i_idx in range(len(dm.time_list)):
                target_count = dm.density_map.get((c_idx, d_idx, i_idx), 0)
                
                existing_count = sum(
                    count for (v, d, i), count in dm.existing_venue_usage.items()
                    if d == d_idx and i == i_idx and dm.venue_idx_to_campus_idx[v] == c_idx
                )
                
                new_count = sum(
                    var for key, var in x.items()
                    if dm.venue_idx_to_campus_idx[key[4]] == c_idx and key[2] == d_idx and key[3] == i_idx
                )
                
                actual_count = new_count + existing_count
                
                diff_var = model.NewIntVar(0, 100, f'diff_c{c_idx}_d{d_idx}_i{i_idx}')
                model.Add(target_count - actual_count <= diff_var)
                model.Add(actual_count - target_count <= diff_var)
                diff_vars.append(diff_var)

    max_deviation = model.NewIntVar(0, 100, 'max_deviation')
    model.AddMaxEquality(max_deviation, diff_vars)

    total_deviation = sum(diff_vars)
    
    WEIGHT_FOR_MAX_DEVIATION = 10000 

    model.Minimize(total_deviation + WEIGHT_FOR_MAX_DEVIATION * max_deviation)
    
    solver = cp_model.CpSolver()
    solver.parameters.max_time_in_seconds = 60.0
    status = solver.Solve(model)

    if status == cp_model.OPTIMAL or status == cp_model.FEASIBLE:
        print(f'Solution found with status: {"OPTIMAL" if status == cp_model.OPTIMAL else "FEASIBLE"}')
        print(f'Objective value (total density difference): {solver.ObjectiveValue()}')
        
        new_schedules = {}
        for key, var in x.items():
            if solver.Value(var) == 1:
                t_idx, c_idx, d_idx, i_idx, v_idx = key
                
                teacher_id = dm.teacher_list[t_idx]['id']
                venue = dm.venue_list[v_idx]

                new_schedule_item = {
                    "id": str(uuid.uuid4()),
                    "day_id": dm.day_list[d_idx]['id'],
                    "time_id": dm.time_list[i_idx]['id'],
                    "course_id": dm.course_list[c_idx]['id'],
                    "campus_id": venue['campus_id'],
                    "venue_id": venue['id']
                }
                
                if teacher_id not in new_schedules:
                    new_schedules[teacher_id] = []
                new_schedules[teacher_id].append(new_schedule_item)
        
        for teacher in dm.data['teachers']:
            if teacher['id'] in new_schedules:
                teacher['scheduled'].extend(new_schedules[teacher['id']])
                print(f"Scheduled {len(new_schedules[teacher['id']])} new class(es) for {teacher['name']}.")

        return dm.data

    else:
        print('No solution found.')
        return None


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Solve a course scheduling problem.')
    parser.add_argument('input_file', type=str, help='Path to the input data JSON file.')
    parser.add_argument('output_file', type=str, help='Path to write the output solution JSON file.')
    args = parser.parse_args()

    try:
        with open(args.input_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except FileNotFoundError:
        print(f"Error: Input file not found at {args.input_file}")
        exit(1)

    data_manager = DataManager(data)
    
    solution_data = solve_scheduling(data_manager)

    if solution_data:
        with open(args.output_file, 'w', encoding='utf-8') as f:
            json.dump(solution_data, f, ensure_ascii=False, indent=2)
        print(f"\nSuccessfully generated schedule. See {args.output_file} for details.")
    else:
        print("\nSolver did not find a solution.")